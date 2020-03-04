use crate::component::ComponentBundle;
use crate::entity::Entity;
use crate::Component;
use ahash::AHashMap;
use bitvec::order::Local;
use bitvec::vec::BitVec;
use erasable::{Erasable, ErasedPtr};
use std::any::TypeId;
use std::iter;
use std::mem::MaybeUninit;
use thiserror::Error;

type StdResult<T, E> = std::result::Result<T, E>;
pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("entity no longer exists")]
    VersionMismatch,
    #[error("entity does not have this component")]
    ComponentNotFound,
}

pub struct World {
    /// Vector of current entity versions, indexed
    /// by the entity's index.
    versions: Vec<u32>,
    /// Bit vector with bits set to 1 for entities that are alive.
    alive: BitVec<Local, usize>,
    /// Mapping from component type IDs to pointers to component storages.
    components: AHashMap<TypeId, ErasedPtr>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            versions: vec![],
            alive: BitVec::new(),
            components: AHashMap::new(),
        }
    }

    pub fn spawn(&mut self, components: impl ComponentBundle) -> Entity {
        let index = self
            .alive
            .iter()
            .enumerate()
            .find_map(|(i, x)| if !*x { Some(i) } else { None })
            .unwrap_or_else(|| {
                let index = self.versions.len();
                self.alloc_more_entities();
                index
            }) as u32;

        self.versions[index as usize] += 1;
        self.alive.set(index as usize, true);
        let version = self.versions[index as usize];

        let entity = Entity { index, version };

        components.add_to(self, entity);

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<()> {
        self.check_valid_entity(entity)?;
        self.alive.set(entity.index as usize, false);
        Ok(())
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.alive[entity.index as usize]
    }

    pub fn get<C: Component>(&self, entity: Entity) -> Result<&C> {
        self.check_valid_entity(entity)?;

        let storage = self
            .component_storage::<C>()
            .ok_or(Error::ComponentNotFound)?;

        storage
            .get(entity.index as usize)
            .ok_or(Error::ComponentNotFound)
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Result<&mut C> {
        self.check_valid_entity(entity)?;

        let storage = self
            .component_storage::<C>()
            .ok_or(Error::ComponentNotFound)?;

        storage
            .get_mut(entity.index as usize)
            .ok_or(Error::ComponentNotFound)
    }

    pub fn add<C: Component>(&mut self, entity: Entity, component: C) -> Result<()> {
        self.check_valid_entity(entity)?;

        let storage = self
            .component_storage_mut::<C>()
            .ok_or(Error::ComponentNotFound)?;

        storage.insert(entity.index as usize, component);

        Ok(())
    }

    pub fn remove<C: Component>(&mut self, entity: Entity) -> Result<()> {
        self.check_valid_entity(entity)?;

        let storage = self
            .component_storage_mut::<C>()
            .ok_or(Error::ComponentNotFound)?;

        storage.remove(entity.index as usize);

        Ok(())
    }

    pub fn size(&self) -> usize {
        self.versions.len()
    }

    fn alloc_more_entities(&mut self) {
        const TO_ADD: usize = 64;
        self.alive.extend(iter::repeat(false).take(TO_ADD));
        self.versions.extend(iter::repeat(0).take(TO_ADD));
    }

    fn check_valid_entity(&self, entity: Entity) -> Result<()> {
        if self.is_alive(entity) && self.versions[entity.index as usize] == entity.version {
            Ok(())
        } else {
            Err(Error::VersionMismatch)
        }
    }

    fn component_storage<C: Component>(&self) -> Option<&ComponentStorage<C>> {
        self.components
            .get(&TypeId::of::<C>())
            .map(|ptr| unsafe { &*ComponentStorage::<C>::unerase(*ptr).as_ptr() })
    }

    fn component_storage_mut<C: Component>(&mut self) -> Option<&mut ComponentStorage<C>> {
        self.components
            .get_mut(&TypeId::of::<C>())
            .map(|ptr| unsafe { &mut *ComponentStorage::<C>::unerase(*ptr).as_ptr() })
    }
}

/// Stores components associated with entities.
///
/// This storage is based on sparse sets.
struct ComponentStorage<T> {
    /// Stores indices into `dense`, indexed by the entity's
    /// index.
    sparse: Vec<u32>,
    /// Stores indices into `sparse`, indexed by the entity's entry
    /// in `sparse`. If the value in this array points to the corresponding
    /// value in `sparse`, then the entity has this component, and the component
    /// is located at the same index in `data`.
    dense: Vec<u32>,
    /// Stores free indices into `dense`. If none are avaible,
    /// `dense` should be extended.
    free: Vec<u32>,
    /// Raw component data.
    data: Vec<MaybeUninit<T>>,
}

impl<T> ComponentStorage<T> {
    fn new(world: &World) -> Self {
        Self {
            sparse: vec![std::u32::MAX; world.size()],
            dense: vec![std::u32::MAX; world.size()],
            free: iter::successors(Some(0), |x| Some(*x + 1))
                .take(world.size())
                .collect(),
            data: iter::repeat_with(|| MaybeUninit::uninit())
                .take(world.size())
                .collect(),
        }
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.get_ptr(index).map(|ptr| unsafe { &*ptr })
    }

    fn get_mut(&self, index: usize) -> Option<&mut T> {
        self.get_ptr(index).map(|ptr| unsafe { &mut *ptr })
    }

    fn get_ptr(&self, index: usize) -> Option<*mut T> {
        if index >= self.sparse.len() {
            return None;
        }

        let dense_index = self.sparse[index] as usize;
        let entity_stored = self.dense[dense_index] as usize;

        if entity_stored != index {
            None
        } else {
            Some(self.data[dense_index].as_ptr() as *mut T)
        }
    }

    fn insert(&mut self, index: usize, component: T) {
        self.extend_if_necessary(index);

        let dense_index = self.find_new_dense_index();

        self.sparse[index] = dense_index;
        self.dense[dense_index as usize] = index as u32;
        self.data[dense_index as usize] = MaybeUninit::new(component);
    }

    fn remove(&mut self, index: usize) -> bool {
        let ptr = match self.get_ptr(index) {
            Some(ptr) => ptr,
            None => return false,
        };

        unsafe {
            let comp = std::ptr::read(ptr);
            drop(comp);
        }

        let dense_index = self.sparse[index];

        self.sparse[index] = std::u32::MAX;
        self.dense[dense_index as usize] = std::u32::MAX;

        self.free.push(dense_index);

        true
    }

    fn find_new_dense_index(&mut self) -> u32 {
        if self.free.is_empty() {
            self.dense.push(std::u32::MAX);
            (self.dense.len() - 1) as u32
        } else {
            self.free.remove(0)
        }
    }

    fn extend_if_necessary(&mut self, index: usize) {
        if index > self.sparse.len() {
            let to_add = self.sparse.len() - index + 1;
            self.sparse.extend(iter::repeat(std::u32::MAX).take(to_add));
        }
    }
}
