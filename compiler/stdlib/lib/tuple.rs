//! Trait implementations for tuples (arity 1 to 12).

use crate::cmp::Ordering;
use crate::fmt;
use crate::hash::{Hash, Hasher};

macro_rules! __std_tuple_impls {
    ($( ( $($T:ident $idx:tt),+ ) )+) => {
        $(
            impl<$($T: Clone),+> Clone for ($($T,)+) {
                #[inline]
                fn clone(&self) -> Self {
                    ($(self.$idx.clone(),)+)
                }
            }

            impl<$($T: PartialEq),+> PartialEq for ($($T,)+) {
                #[inline]
                fn eq(&self, other: &($($T,)+)) -> bool {
                    true $(&& self.$idx == other.$idx)+
                }
                #[inline]
                fn ne(&self, other: &($($T,)+)) -> bool {
                    false $(|| self.$idx != other.$idx)+
                }
            }

            impl<$($T: Eq),+> Eq for ($($T,)+) {}

            impl<$($T: PartialOrd),+> PartialOrd for ($($T,)+) {
                #[inline]
                fn partial_cmp(&self, other: &($($T,)+)) -> Option<Ordering> {
                    $(
                        match self.$idx.partial_cmp(&other.$idx) {
                            Some(Ordering::Equal) => {}
                            ordering => return ordering,
                        }
                    )+
                    Some(Ordering::Equal)
                }
            }

            impl<$($T: Ord),+> Ord for ($($T,)+) {
                #[inline]
                fn cmp(&self, other: &($($T,)+)) -> Ordering {
                    $(
                        match self.$idx.cmp(&other.$idx) {
                            Ordering::Equal => {}
                            ordering => return ordering,
                        }
                    )+
                    Ordering::Equal
                }
            }

            impl<$($T: Default),+> Default for ($($T,)+) {
                #[inline]
                fn default() -> ($($T,)+) {
                    ($($T::default(),)+)
                }
            }

            impl<$($T: Hash),+> Hash for ($($T,)+) {
                #[inline]
                fn hash<S: Hasher>(&self, state: &mut S) {
                    $(self.$idx.hash(state);)+
                }
            }

            impl<$($T: fmt::Debug),+> fmt::Debug for ($($T,)+) {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    let mut builder = f.debug_tuple("");
                    $(builder.field(&self.$idx);)+
                    builder.finish()
                }
            }
        )+
    };
}

__std_tuple_impls! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
    (A 0, B 1, C 2, D 3, E 4)
    (A 0, B 1, C 2, D 3, E 4, F 5)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10)
    (A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11)
}
