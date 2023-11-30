use crate::AtomicBox;

pub struct AtomicList<T> {
    head: AtomicBox<Node<T>>,
}

struct Node<T> {
    elem: T,
    next: AtomicBox<Node<T>>,
}

impl<T> AtomicList<T> {
    pub fn empty(&self) -> bool {
        self.head.is_null()
    }

    pub fn push_front(&mut self, elem: T) {
        let node = Box::new(Node {
            elem,
            next: AtomicBox::invalid(),
        });
        self.head.swap_and(node, |new, old| {
            if let Some(next) = old {
                new.next.swap(next);
            } else {
                new.next.take();
            }
        });
    }
}

impl<T> Into<Vec<T>> for AtomicList<T> {
    fn into(mut self) -> Vec<T> {
        let mut head = self.head.take();
        let mut vec = Vec::<T>::new();
        while let Some(mut node) = head {
            vec.push(node.elem);
            head = node.next.take();
        }
        vec
    }
}

impl<T> Default for AtomicList<T> {
    fn default() -> Self {
        Self {
            head: Default::default(),
        }
    }
}

impl<T> Drop for AtomicList<T> {
    fn drop(&mut self) {
        let mut head = self.head.take();
        while let Some(mut node) = head {
            head = node.next.take();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn normal() {
        let mut l: AtomicList<u32> = Default::default();
        assert!(l.empty());

        l.push_front(0u32);
        assert!(!l.empty());

        l.push_front(1u32);
        assert!(!l.empty());

        let v: Vec<u32> = l.into();
        assert_eq!(v, vec![1u32, 0u32]);
    }

    #[test]
    fn push_one_million() {
        let mut l: AtomicList<i32> = Default::default();
        for i in 0..1000000 {
            l.push_front(i);
        }
    }
}
