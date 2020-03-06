use crate::component::ComponentBundle;
use crate::entity::Entity;
use crate::Component;
use ahash::AHashMap;
use bitvec::order::Local;
use bitvec::vec::BitVec;
use erasable::{Erasable, ErasedPtr};
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::ptr::NonNull;
use std::{iter, ptr};
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
    /// Set of free entity indices.
    free: Vec<u32>,
    /// Mapping from component type IDs to pointers to component storages.
    components: AHashMap<TypeId, ErasedPtr>,
    /// Next entity index to add.
    entity_counter: u32,
    /// Number of entities in the world.
    num_entities: usize,
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
            entity_counter: 0,
            free: vec![],
            num_entities: 0,
        }
    }

    pub fn spawn(&mut self, components: impl ComponentBundle) -> Entity {
        let index = if self.free.is_empty() {
            if self.entity_counter >= self.versions.len() as u32 {
                self.alloc_more_entities();
            }
            let i = self.entity_counter;
            self.entity_counter += 1;
            i
        } else {
            self.free.remove(0)
        };

        self.versions[index as usize] += 1;
        self.alive.set(index as usize, true);
        let version = self.versions[index as usize];

        let entity = Entity { index, version };

        components
            .add_to(self, entity)
            .expect("components failed to add");

        self.num_entities += 1;

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<()> {
        self.check_valid_entity(entity)?;
        self.alive.set(entity.index as usize, false);
        self.num_entities -= 1;
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

        let storage = match self.component_storage_mut::<C>() {
            Some(storage) => storage,
            None => self.create_new_storage(),
        };

        storage.insert(entity.index as usize, component);

        Ok(())
    }

    pub fn remove<C: Component>(&mut self, entity: Entity) -> Result<()> {
        self.check_valid_entity(entity)?;

        let storage = self
            .component_storage_mut::<C>()
            .ok_or(Error::ComponentNotFound)?;

        if !storage.remove(entity.index as usize) {
            return Err(Error::ComponentNotFound);
        }

        Ok(())
    }

    pub fn size(&self) -> usize {
        self.num_entities
    }

    pub fn clear(&mut self) {
        self.versions.clear();
        self.alive.clear();
        // TODO: proper storage clear
        self.components.clear();
        self.num_entities = 0;
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

    fn create_new_storage<C: Component>(&mut self) -> &mut ComponentStorage<C> {
        let storage = Box::new(ComponentStorage::<C>::new(self));

        self.components.insert(
            TypeId::of::<C>(),
            erasable::erase(NonNull::new(Box::into_raw(storage)).unwrap()),
        );

        self.component_storage_mut().unwrap()
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
    /// Raw component data.
    data: Vec<UnsafeCell<T>>,
}

impl<T> ComponentStorage<T> {
    fn new(world: &World) -> Self {
        Self {
            sparse: vec![std::u32::MAX; world.size()],
            dense: vec![],
            data: vec![],
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
            Some(self.data[dense_index].get())
        }
    }

    fn insert(&mut self, index: usize, component: T) {
        self.extend_if_necessary(index);

        let dense_index = self.dense.len() as u32;
        self.dense.push(index as u32);
        self.data.push(UnsafeCell::new(component));

        self.sparse[index] = dense_index;
    }

    fn remove(&mut self, index: usize) -> bool {
        let ptr = match self.get_ptr(index) {
            Some(ptr) => ptr,
            None => return false,
        };

        // Swap-remove the component + entry in dense array
        let dense_index = self.sparse[index];

        if self.dense.len() == 1 {
            self.dense.clear();
            self.data.clear();
        } else {
            self.dense.swap_remove(dense_index as usize);
            self.data.swap_remove(dense_index as usize);
        }

        self.sparse[self.dense[dense_index as usize] as usize] = dense_index;

        true
    }

    fn clear(&mut self) {
        // Drop all components
        for (index, dense_index) in self.sparse.iter().copied().enumerate() {
            if self.dense[dense_index as usize] == index as u32 {
                unsafe {
                    let _comp = ptr::read(self.data[dense_index as usize].get());
                }
            }
        }

        self.sparse
            .iter_mut()
            .chain(self.dense.iter_mut())
            .for_each(|x| *x = std::u32::MAX);
    }

    fn extend_if_necessary(&mut self, index: usize) {
        if index >= self.sparse.len() {
            let to_add = index - self.sparse.len() + 1;
            self.sparse.extend(iter::repeat(std::u32::MAX).take(to_add));
        }
    }
}
