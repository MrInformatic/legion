use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    hash::BuildHasherDefault,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{
    hash::U64Hasher,
    storage::{archetype::ArchetypeIndex, ComponentIndex},
};

/// An opaque identifier for an entity.
#[derive(Debug, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Entity(NonZeroU64);

thread_local! {
    pub static ID_CLONE_MAPPINGS: RefCell<HashMap<Entity, Entity, EntityHasher>> = RefCell::new(HashMap::default());
}

impl Clone for Entity {
    fn clone(&self) -> Self {
        ID_CLONE_MAPPINGS.with(|cell| {
            let map = cell.borrow();
            *map.get(self).unwrap_or(self)
        })
    }
}

const BLOCK_SIZE: u64 = 16;

// Always divisible by BLOCK_SIZE.
// Safety: This must never be 0, so skip the first block
static NEXT_ENTITY: AtomicU64 = AtomicU64::new(BLOCK_SIZE);

/// An iterator which yields new entity IDs.
#[derive(Debug)]
pub struct Allocate {
    next: u64,
}

impl Allocate {
    /// Constructs a new enity ID allocator iterator.
    pub fn new() -> Self {
        // This is still safe because the allocator grabs a new block immediately
        Self { next: 0 }
    }
}

impl Default for Allocate {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Iterator for Allocate {
    type Item = Entity;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.next % BLOCK_SIZE == 0 {
            // This is either the first block, or we overflowed to the next block.
            self.next = NEXT_ENTITY.fetch_add(BLOCK_SIZE, Ordering::Relaxed);
            debug_assert_eq!(self.next % BLOCK_SIZE, 0);
        }

        // Safety: self.next can't be 0 as long as the first block is skipped,
        // and no overflow occurs in NEXT_ENTITY
        let entity = unsafe {
            debug_assert_ne!(self.next, 0);
            Entity(NonZeroU64::new_unchecked(self.next))
        };
        self.next += 1;
        Some(entity)
    }
}

/// The storage location of an entity's data.
#[derive(Debug, Copy, Clone)]
pub struct EntityLocation(pub(crate) ArchetypeIndex, pub(crate) ComponentIndex);

impl EntityLocation {
    /// Constructs a new entity location.
    pub fn new(archetype: ArchetypeIndex, component: ComponentIndex) -> Self {
        EntityLocation(archetype, component)
    }

    /// Returns the entity's archetype index.
    pub fn archetype(&self) -> ArchetypeIndex {
        self.0
    }

    /// Returns the entity's component index within its archetype.
    pub fn component(&self) -> ComponentIndex {
        self.1
    }
}

/// A hasher optimized for entity IDs.
pub type EntityHasher = BuildHasherDefault<U64Hasher>;

/// A map of entity IDs to their storage locations.
#[derive(Clone, Default)]
pub struct LocationMap(HashMap<Entity, EntityLocation, EntityHasher>);

impl Debug for LocationMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.0.iter()).finish()
    }
}

impl LocationMap {
    /// Returns the number of entities in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the location map is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the location map contains the given entity.
    pub fn contains(&self, entity: Entity) -> bool {
        self.0.contains_key(&entity)
    }

    /// Inserts a collection of adjacent entities into the location map.
    pub fn insert(
        &mut self,
        ids: &[Entity],
        arch: ArchetypeIndex,
        ComponentIndex(base): ComponentIndex,
    ) -> Vec<EntityLocation> {
        ids.iter().enumerate()
            .filter_map(|(i, entity)| self.0.insert(*entity, EntityLocation::new(arch, ComponentIndex(base + i))))
            .collect()
    }

    /// Inserts or updates the location of an entity.
    pub fn set(&mut self, entity: Entity, location: EntityLocation) {
        self.0.insert(entity, location);
    }

    /// Returns the location of an entity.
    pub fn get(&self, entity: Entity) -> Option<EntityLocation> {
        self.0.get(&entity).cloned()
    }

    /// Removes an entity from the location map.
    pub fn remove(&mut self, entity: Entity) -> Option<EntityLocation> {
        self.0.remove(&entity)
    }
}
