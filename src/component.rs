use std::any::{Any, TypeId};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::mem;

pub struct ComponentStorage<T> {
    components: Vec<Option<T>>,
}

impl<T> ComponentStorage<T> {
    pub fn new() -> ComponentStorage<T> {
        ComponentStorage {
            components: Vec::new(),
        }
    }

    pub fn contains(&self, entity: usize) -> bool {
        self.components
            .get(entity)
            .map(|c| c.is_some())
            .unwrap_or(false)
    }

    pub fn insert(&mut self, entity: usize, c: T) {
        if entity >= self.components.len() {
            self.components.resize_with(entity, || None);
        }
        self.components.insert(entity, Some(c));
    }

    pub fn remove(&mut self, entity: usize) -> Option<T> {
        self.components.get_mut(entity).and_then(|v| v.take())
    }

    pub fn get(&self, entity: usize) -> Option<&T> {
        self.components.get(entity).and_then(|v| v.as_ref())
    }

    pub fn get_mut(&mut self, entity: usize) -> Option<&mut T> {
        self.components.get_mut(entity).and_then(|v| v.as_mut())
    }
}

pub trait GenericComponentStorage {
    fn next_entry(&self, start: usize) -> Option<usize>;
    fn remove(&mut self, id: usize);
    fn as_any(&self) -> &Any;
    fn as_any_mut(&mut self) -> &mut Any;
}

impl<T: 'static> GenericComponentStorage for ComponentStorage<T> {
    fn next_entry(&self, start: usize) -> Option<usize> {
        self.components.get(start..).and_then(|s| {
            s.iter()
                .enumerate()
                .filter_map(|(e, c)| if c.is_some() { Some(start + e) } else { None })
                .next()
        })
    }
    fn remove(&mut self, id: usize) {
        self.remove(id);
    }
    fn as_any(&self) -> &Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}

pub trait ComponentSet<'a> {
    type Refs;

    fn iter(
        storage: &'a HashMap<TypeId, RefCell<Box<GenericComponentStorage>>>,
    ) -> Box<Iterator<Item = (usize, Self::Refs)> + 'a>;
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! implement_tuple_set {
    ($($x:ident:$xn:ident),*) => {
        impl<'a, $($x: 'static,)*> ComponentSet<'a> for ($($x,)*) {
            type Refs = ($(&'a mut $x,)*);

            fn iter(
                storage: &'a HashMap<TypeId, RefCell<Box<GenericComponentStorage>>>
            ) -> Box<Iterator<Item = (usize, Self::Refs)> + 'a> {

                struct ComponentIterator<'a, $($x: 'a),*> {
                    index: usize,
                    $($xn: (RefMut<'a, ComponentStorage<$x>>)),*
                }
                impl<'a, $($x: 'static),*> Iterator for ComponentIterator<'a, $($x),*> {
                    type Item = (usize, ($(&'a mut $x,)*));

                    fn next(&mut self) -> Option<Self::Item> {
                        let component_count = 0 $(+ replace_expr!($x 1))*;
                        let mut entity = self.index;
                        let mut entity_count = 0;
                        let next_entity = loop {
                            $(
                                if let Some(e) = self.$xn.next_entry(entity) {
                                    if e != entity {
                                        entity_count = 0;
                                    }
                                    entity_count += 1;
                                    entity = e;
                                } else {
                                    break None;
                                }

                                if entity_count == component_count {
                                    break Some(entity);
                                }
                            )*
                            entity += 1;
                        };

                        if let Some(e) = next_entity {
                            self.index = e + 1;

                            // we can transmute the lifetime of the references to the lifetime of the iterator because:
                            // * this iterator holds a mutable reference to the component storage, guaranteeing there are no
                            //   other references to the storage or any component entry in the storage
                            // * the iterator can return only one mutable reference to each unique component entry
                            unsafe {
                                Some((
                                    e,
                                    (
                                        $(mem::transmute::<&mut $x, &'a mut $x>(self.$xn.get_mut(e).unwrap()),)+
                                    )
                                ))
                            }
                        } else {
                            None
                        }

                    }
                }

                Box::new(
                    ComponentIterator {
                        index: 0,
                        $($xn: RefMut::map(
                            storage.get(&TypeId::of::<$x>()).expect("component not registered").borrow_mut(),
                            |s| s.as_any_mut().downcast_mut::<ComponentStorage<$x>>().unwrap()
                        )),*
                    }
                )
            }
        }
    }
}
implement_tuple_set! {A:a}
implement_tuple_set! {A:a, B:b}
implement_tuple_set! {A:a, B:b, C:c}
implement_tuple_set! {A:a, B:b, C:c, D:d}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g, H:h}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g, H:h, I:i}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g, H:h, I:i, J:j}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g, H:h, I:i, J:j, K:k}
implement_tuple_set! {A:a, B:b, C:c, D:d, E:e, F:f, G:g, H:h, I:i, J:j, K:k, L:l}
