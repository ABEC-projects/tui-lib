use std::any::Any;

use super::anyvec::AnyVec;


pub struct AnyArena {
    items: AnyVec
}

#[derive(Debug, Clone)]
pub struct ArenaItemAny <T> {
    inner: T,
    alive: bool,
    generation: usize,
}

impl <T> ArenaItemAny <T> {
    fn new (item: T) -> Self {
        Self { inner: item, alive: true, generation: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct ArenaHandleAny <T> {
    index: usize,
    generation: usize,
    _marker: std::marker::PhantomData<T>,
}

impl <T> ArenaHandleAny<T> {
    fn new (index: usize, generation: usize) -> Self {
        Self { index, generation, _marker: std::marker::PhantomData }
    }
}

impl AnyArena {
    
    pub fn new <T: Any> () -> Self {
        Self { items: AnyVec::new::<ArenaItemAny<T>>() }
    }

    pub fn insert <T: Any> (&mut self, item: T) -> ArenaHandleAny<T> {
        let mut found = false;
        let mut index = 0;
        for (i, x) in self.items.slice::<ArenaItemAny<T>>().iter().enumerate() {
            if !x.alive {
                found = true;
                index = i;
                break;
            }
        }
        if !found {
            self.items.push(ArenaItemAny::new(item));
            ArenaHandleAny::new(self.items.len() - 1, 0)
        } else {
            let it = &mut self.items.slice_mut::<ArenaItemAny<T>>()[index];
            it.generation += 1;
            it.alive = true;
            it.inner = item;
            ArenaHandleAny::new(index, it.generation)
        }
    }

    pub fn get <T: Any> (&self, handle: &ArenaHandleAny<T>) -> Option<&T> {
        let item: &ArenaItemAny<T> = self.items.slice().get(handle.index)?;
        if item.generation == handle.generation && item.alive {
            Some(&item.inner)
        } else {
            None
        }
    }

    pub fn get_mut <T: Any> (&mut self, handle: &ArenaHandleAny<T>) -> Option<&mut T> {
        let item: &mut ArenaItemAny<T> = self.items.slice_mut().get_mut(handle.index)?;
        if item.generation == handle.generation && item.alive {
            Some(&mut item.inner)
        } else {
            None
        }
    }

    pub fn remove <T: Any> (&mut self, handle: ArenaHandleAny<T>) {
        self.items.slice_mut::<ArenaItemAny<T>>()[handle.index].alive = false;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test(){
        let mut aa = AnyArena::new::<usize>();
        let h1 = aa.insert(0_usize);
        let h2 = aa.insert(1_usize);
        let h3 = aa.insert(2_usize);
        aa.remove(h2.clone());
        assert_eq!(*aa.get::<usize>(&h1).unwrap(), 0_usize);
        assert_eq!(aa.get::<usize>(&h2), None);
        assert_eq!(*aa.get::<usize>(&h3).unwrap(), 2_usize);
        assert!(!aa.items.slice::<ArenaItemAny<usize>>()[1].alive);
    }
}
