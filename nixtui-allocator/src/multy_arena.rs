mod anyvec;
pub mod anyarena;
use std::{any::{Any, TypeId}, collections::HashMap};
use anyarena::{AnyArena, ArenaHandleAny as MultyArenaHandle};


pub struct MultyArena {
    map: HashMap<TypeId, AnyArena>,
}

impl MultyArena {

    fn get_aa <T: Any> (&self) -> Option<&AnyArena> {
        self.map.get(&TypeId::of::<T>())
    }

    fn get_aa_mut <T: Any> (&mut self) -> Option<&mut AnyArena> {
        self.map.get_mut(&TypeId::of::<T>())
    }

    pub fn new() -> Self {
        let map = HashMap::new();
        Self { map }
    }

    pub fn register <T: Any>(&mut self) {
        self.map.insert(TypeId::of::<T>(), AnyArena::new::<T>());
    }

    pub fn get <T: Any> (&self, handle: &MultyArenaHandle<T>) -> Option<&T> {
        self.map.get(&TypeId::of::<T>())?.get(handle)
    }

    /// # Panics
    /// Will panic if type `T` is not registred first using `register()`
    pub fn insert <T: Any> (&mut self, item: T) -> MultyArenaHandle<T> {
        self.get_aa_mut::<T>().unwrap().insert(item)
    }
}

impl Default for MultyArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Test (usize);

    #[test]
    fn test_get() {
        let mut ma = MultyArena::new();
        ma.register::<Test>();
        let h = ma.insert(Test(12));
        let i = ma.get(&h).unwrap();
        assert_eq!(*i, Test(12));
    }
}
