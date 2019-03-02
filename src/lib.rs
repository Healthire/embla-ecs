//! # lil-ecs
//!
//! lil-ecs is a little entity component system with a little API.
//!
//! It is intended to be used for small jam-style games, and specifically for wasm projects.As such, the focus is not primarily on performance, but on providing a simple API that's easy to iterate with.
//!
//! ## lil-ecs should be:
//! * Simple to use
//! * Single-threaded
//! * Reasonably performant
//!
//! ## lil-ecs is *not* intended to be:
//! * as featureful as other more mature ECS libraries
//! * as performant as other ECS libraries
//! * multi-threaded (until supported by wasm)
//!
//! ## panics
//! To keep the API simple, lil-ecs will currently panic if components are locked in multiple places. To avoid encountering this:
//!  * Don't keep multiple iterators over overlapping component sets
//!  * Don't insert or remove components while iterating over components
//!
//! lil-ecs will also panic if any attempt is made to insert, remove, or iterate on an unregistered component

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt;

mod component;

use component::{ComponentSet, ComponentStorage, GenericComponentStorage};

/// Error type of lil-ecs.
///
/// A very simple error type with only a few kinds of errors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// Entity is no longer alive.
    ///
    /// Error triggered on any attempt to access an entity that has previously been removed.
    DeadEntityAccess(Entity),

    /// The component has not been registered.
    ///
    /// An error triggered on trying to read, insert, or remove a component that has not been registered in the World.
    UnregisteredComponentAccess,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::DeadEntityAccess(e) => write!(f, "Entity '{}' is no longer alive", e),
            Error::UnregisteredComponentAccess => {
                write!(f, "Attempt to access unregistered component")
            }
        }
    }
}

/// A unique reference to an entity living in the World
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    index: usize,
    generation: u32,
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.index, self.generation)
    }
}

/// An entry to an entity in its associated World
///
/// Used to insert or remove components for an entity
#[derive(Clone, Copy)]
pub struct EntityEntry<'a> {
    world: &'a World,
    e: Entity,
}

impl<'a> EntityEntry<'a> {
    pub fn entity(&self) -> Entity {
        self.e
    }

    /// Inserts a component to the Entity, replacing any existing component
    ///
    /// # Panics
    /// Panics if the component is not registered, or if the component is already locked elsewhere.
    pub fn insert<C: 'static>(&self, c: C) -> Result<&Self, Error> {
        self.world.insert_component(self.e, c)?;
        Ok(self)
    }

    /// Removes a component from the Entity
    ///
    /// Returns the component that was removed, if any.
    ///
    /// # Panics
    /// Panics if the component is not registered, or if the component is already locked elsewhere.
    pub fn remove<C: 'static>(&self) -> Result<Option<C>, Error> {
        Ok(self.world.remove_component::<C>(self.e)?)
    }
}

/// A collection of entities and components
///
///
/// # Usage:
///
/// ```rust
/// use lil_ecs::{World, Error};
///
/// struct Position(i32, i32);
/// struct Velocity(i32, i32);
///
/// let mut world = World::new();
///
/// world.register_component::<Position>();
/// world.register_component::<Velocity>();
///
/// let e1 = world
///     .add_entity()
///     .insert(Position(0, 0)).unwrap()
///     .insert(Velocity(5, 5)).unwrap()
///     .entity();
/// let e2 = world
///     .add_entity()
///     .insert(Position(0, 0)).unwrap()
///     .insert(Velocity(3, 4)).unwrap()
///     .entity();
/// let e3 = world.add_entity().insert(Position(0, 0)).unwrap().entity();
///
/// for (entity, (mut pos,)) in world.iter_entities::<(Position,)>() {
///     pos.0 = 10;
///     if entity == e1 {
///         pos.0 = 0;
///     }
/// }
///
/// for (entity, (mut pos, vel)) in world.iter_entities::<(Position, Velocity)>() {
///     pos.0 += vel.0;
///     pos.1 += vel.1;
///     if entity == e1 {
///         pos.0 = 0;
///     }
/// }
///
/// for (mut pos,) in world.iter::<(Position,)>() {
///     pos.0 = 5;
/// }
///
/// world.remove_entity(e1);
/// world.remove_entity(e2);
/// world.remove_entity(e3);
/// ```
pub struct World {
    components: HashMap<TypeId, RefCell<Box<GenericComponentStorage>>>,
    entities: Vec<u32>,
    dead: Vec<usize>,
}

impl World {
    pub fn new() -> World {
        World {
            components: HashMap::new(),
            entities: Vec::new(),
            dead: Vec::new(),
        }
    }

    /// Registers a component for use in the world.
    ///
    /// Any type can be registered as a component. Components are indexed by type, meaning each component must to be a *unique* type.
    pub fn register_component<C: 'static>(&mut self) {
        self.components.insert(
            TypeId::of::<C>(),
            RefCell::new(Box::new(ComponentStorage::<C>::new())),
        );
    }

    /// Gets an EntityEntry for the provided Entity
    pub fn entity<'a>(&'a mut self, e: Entity) -> Option<EntityEntry<'a>> {
        match self.entities.get(e.index) {
            Some(&gen) if gen == e.generation => Some(EntityEntry { world: self, e: e }),
            _ => None,
        }
    }

    /// Allocates a new Entity and returns an EntityEntry for the newly created Entity
    pub fn add_entity<'a>(&'a mut self) -> EntityEntry<'a> {
        let entity_id = if let Some(index) = self.dead.pop() {
            let generation = self.entities[index] + 1;
            self.entities[index] = generation;
            Entity { index, generation }
        } else {
            let index = self.entities.len();
            self.entities.push(0);
            Entity {
                index,
                generation: 0,
            }
        };
        EntityEntry {
            world: self,
            e: entity_id,
        }
    }

    /// Removes an Entity
    pub fn remove_entity(&mut self, e: Entity) {
        match self.entities.get(e.index) {
            Some(&gen) if gen == e.generation => {
                for (_, storage) in self.components.iter() {
                    storage.borrow_mut().remove(e.index);
                }
                self.dead.push(e.index);
            }
            _ => {}
        }
    }

    /// Gets a specific component from an entity
    ///
    /// Returns an Error::DeadEntityAccess error if the entity is not alive
    /// Successful result is Some(Ref<C>) if the entity has the specified component, otherwise None
    ///
    /// # Panics
    /// Panics if the component is not registered, or if the component is already locked elsewhere.
    pub fn get_component<'b, C: 'static>(&'b self, e: Entity) -> Result<Option<Ref<'b, C>>, Error> {
        match self.entities.get(e.index) {
            Some(&gen) if gen == e.generation => {
                let storage = self.get_storage::<C>()?;
                if storage.contains(e.index) {
                    Ok(Some(Ref::map(storage, |s| s.get(e.index).unwrap())))
                } else {
                    Ok(None)
                }
            }
            Some(_) => Err(Error::DeadEntityAccess(e)),
            _ => Ok(None),
        }
    }

    /// Iterates over a ComponentSet
    ///
    /// ComponentSet is implemented for all tuples from `(A,)` to `(A, B, C, D, E, F, G, H, I, J, K, L)`
    ///
    /// Mutably locks the storage for each component included in the component set.
    ///
    /// # Panics
    /// Panics if any of the components are not registered, or if any of the components are locked elsewhere.
    pub fn iter<'a, C: ComponentSet<'a>>(&'a self) -> Box<Iterator<Item = C::IterItem> + 'a> {
        C::iter(&self.components)
    }

    /// Iterates over a ComponentSet and also provides the Entity for which the components belong
    ///
    /// ComponentSet is implemented for all tuples from `(A,)` to `(A, B, C, D, E, F, G, H, I, J, K, L)`
    ///
    /// Mutably locks the storage for each component included in the component set.
    ///
    /// # Panics
    /// Panics if any of the components are not registered, or if any of the components are locked elsewhere.
    pub fn iter_entities<'a, C: ComponentSet<'a> + 'static>(
        &'a self,
    ) -> Box<Iterator<Item = (Entity, C::IterItem)> + 'a> {
        let entities = &self.entities;
        Box::new(C::indexed(&self.components).map(move |(e, cs)| {
            (
                Entity {
                    index: e,
                    generation: entities[e],
                },
                cs,
            )
        }))
    }

    fn get_storage<T: 'static>(&self) -> Result<Ref<ComponentStorage<T>>, Error> {
        Ok(Ref::map(
            self.components
                .get(&TypeId::of::<T>())
                .ok_or(Error::UnregisteredComponentAccess)?
                .borrow(),
            |s| s.as_any().downcast_ref::<ComponentStorage<T>>().unwrap(),
        ))
    }

    fn get_storage_mut<T: 'static>(&self) -> Result<RefMut<ComponentStorage<T>>, Error> {
        Ok(RefMut::map(
            self.components
                .get(&TypeId::of::<T>())
                .ok_or(Error::UnregisteredComponentAccess)?
                .borrow_mut(),
            |s| {
                s.as_any_mut()
                    .downcast_mut::<ComponentStorage<T>>()
                    .unwrap()
            },
        ))
    }

    fn insert_component<T: 'static>(&self, e: Entity, c: T) -> Result<(), Error> {
        if e.index >= self.entities.len() || e.generation != self.entities[e.index] {
            return Err(Error::DeadEntityAccess(e));
        }
        self.get_storage_mut::<T>()?.insert(e.index, c);
        Ok(())
    }

    fn remove_component<T: 'static>(&self, e: Entity) -> Result<Option<T>, Error> {
        if e.index >= self.entities.len() || e.generation != self.entities[e.index] {
            return Err(Error::DeadEntityAccess(e));
        }
        Ok(self.get_storage_mut::<T>()?.remove(e.index))
    }
}

#[cfg(test)]
mod tests {
    use super::World;

    struct Position(i32, i32);
    struct Velocity(i32, i32);

    #[test]
    fn ecs() {
        let mut world = World::new();

        world.register_component::<Position>();
        world.register_component::<Velocity>();

        let e1 = world
            .add_entity()
            .insert(Position(0, 0))
            .unwrap()
            .insert(Velocity(5, 5))
            .unwrap()
            .entity();
        let e2 = world
            .add_entity()
            .insert(Position(0, 0))
            .unwrap()
            .insert(Velocity(3, 4))
            .unwrap()
            .entity();
        let e3 = world.add_entity().insert(Position(0, 0)).unwrap().entity();

        let position_entities = &[e1, e2, e3];
        for (entity, (mut pos,)) in world.iter_entities::<(Position,)>() {
            pos.0 = 10;
            assert!(position_entities.contains(&entity));
        }

        let velocity_entities = &[e1, e2];
        for (entity, (mut pos, vel)) in world.iter_entities::<(Position, Velocity)>() {
            pos.0 += vel.0;
            pos.1 += vel.1;
            assert!(velocity_entities.contains(&entity));
        }

        assert_eq!(world.get_component::<Position>(e1).unwrap().unwrap().0, 15);
        assert_eq!(world.get_component::<Position>(e2).unwrap().unwrap().0, 13);
        assert_eq!(world.get_component::<Position>(e3).unwrap().unwrap().0, 10);

        world.remove_component::<Position>(e1).unwrap();

        for (mut pos,) in world.iter::<(Position,)>() {
            pos.0 = 5;
        }

        assert_eq!(world.get_component::<Position>(e2).unwrap().unwrap().0, 5);
        assert_eq!(world.get_component::<Position>(e3).unwrap().unwrap().0, 5);

        world.remove_entity(e3);
        let e4 = world.add_entity().entity();
        assert_eq!(e3.index, e4.index);
        assert_ne!(e3, e4);
    }
}
