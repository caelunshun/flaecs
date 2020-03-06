use crate::{Entity, Result, World};

pub trait Component: Send + Sync + 'static {}
impl<T> Component for T where T: Send + Sync + 'static {}

pub trait ComponentBundle {
    fn add_to(self, world: &mut World, entity: Entity) -> Result<()>;
}

impl<A> ComponentBundle for (A,)
where
    A: Component,
{
    fn add_to(self, world: &mut World, entity: Entity) -> Result<()> {
        world.add(entity, self.0)
    }
}

macro_rules! impl_bundle {
    ($($name:ident),* ; $($idx:tt),*) => {
        impl<$($name: Component),*> ComponentBundle for ($($name,)*) {
            fn add_to(self, world: &mut World, entity: Entity) -> Result<()> {
                $(
                    let x = self.$idx;
                    world.add(entity, x)?;
                )*

                Ok(())
            }
        }
    }
}

impl_bundle!(A, B; 0, 1);
impl_bundle!(A, B, C; 0, 1, 2);
impl_bundle!(A, B, C, D; 0, 1, 2, 3);
impl_bundle!(A, B, C, D, E; 0, 1, 2, 3, 4);
impl_bundle!(A, B, C, D, E, F; 0, 1, 2, 3, 4, 5);
impl_bundle!(A, B, C, D, E, F, G; 0, 1, 2, 3, 4, 5, 6);
impl_bundle!(A, B, C, D, E, F, G, H; 0, 1, 2, 3, 4, 5, 6, 7);
impl_bundle!(A, B, C, D, E, F, G, H, I; 0, 1, 2, 3, 4, 5, 6, 7, 8);
impl_bundle!(A, B, C, D, E, F, G, H, I, J; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24);
