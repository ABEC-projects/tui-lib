pub mod multy_arena;
use std::fmt::Debug;



#[derive(Debug, Clone)]
pub struct ArenaAlloc <T> {
    items: Vec<ArenaItem<T>>,
}

#[derive(Debug, Clone)]
pub struct ArenaItem <T> {
    inner: T,
    alive: bool,
    generation: usize,
}

#[derive(Debug, Clone)]
pub struct ArenaHandle <T> {
    index: usize,
    generation: usize,
    _marker: std::marker::PhantomData<T>,
}

impl <T> ArenaHandle<T> {
    fn new(index: usize, generation: usize) -> Self {
        Self {index, generation, _marker: std::marker::PhantomData}
    }
}

impl <T> ArenaItem<T> {
    fn new(item: T) -> Self {
        Self { inner: item, alive: true, generation: 0 }
    }
}

impl <T> ArenaAlloc<T> {
    
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn insert(&mut self, item: T) -> ArenaHandle<T> {
        let mut found = false;
        let mut index = 0;
        for (i, x) in self.items.iter().enumerate() {
            if !x.alive {
                found = true;
                index = i;
                break;
            }
        }
        if !found {
            self.items.push(ArenaItem::new(item));
            ArenaHandle::new(self.items.len() - 1, 0)
        } else {
            let it = &mut self.items[index];
            it.generation += 1;
            it.alive = true;
            it.inner = item;
            ArenaHandle::new(index, it.generation)
        }
    }

    pub fn get(&self, handle: &ArenaHandle<T>) -> Option<&T> {
        let item = self.items.get(handle.index)?;
        if item.generation == handle.generation && item.alive {
            Some(&item.inner)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, handle: &ArenaHandle<T>) -> Option<&mut T> {
        let item = self.items.get_mut(handle.index)?;
        if item.generation == handle.generation && item.alive {
            Some(&mut item.inner)
        } else {
            None
        }
    }

    pub fn remove(&mut self, handle: ArenaHandle<T>) {
        self.items[handle.index].alive = false;
    }
}

impl <T> Default for ArenaAlloc<T> {
    fn default() -> Self {
        Self::new()
    }
}
