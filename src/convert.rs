//! Rust <-> K conversions. `From<rust>` builds a K; `TryFrom<K>` extracts one.

use crate::error::LError;
use crate::k::K;

/// `from!(rust_ty => Variant, …)` — wrap a Rust value straight into a K.
macro_rules! from { ($($t:ty => $v:ident),* $(,)?) => {
    $( impl From<$t> for K { fn from(x: $t) -> K { K::$v(x) } } )*
}}
from! {
    bool => Bool, u8 => Byte, i16 => Short, i32 => Int, i64 => Long,
    f32 => Real, f64 => Float, Vec<bool> => BoolVec, Vec<i32> => IntVec,
    Vec<i64> => LongVec, Vec<f64> => FloatVec, Vec<String> => SymbolVec,
}

// strings -> char vector (L string); symbol lists keep the `Vec<&str>` shape.
impl From<&str> for K {                                                         // borrowed string slice
    fn from(s: &str) -> K { K::CharVec(s.as_bytes().to_vec()) }
}
impl From<String> for K {                                                       // owned string
    fn from(s: String) -> K { K::CharVec(s.into_bytes()) }
}
impl From<Vec<&str>> for K {                                                    // -> SymbolVec
    fn from(v: Vec<&str>) -> K {
        K::SymbolVec(v.iter().map(|s| s.to_string()).collect())
    }
}

/// `acc!(rust_ty => K-accessor / err)` — extract via a widening accessor.
macro_rules! acc { ($($t:ty => $a:ident / $m:literal),* $(,)?) => {
    $( impl TryFrom<K> for $t { type Error = LError;
        fn try_from(k: K) -> Result<$t, LError> {
            k.$a().ok_or(LError::Type($m.into()))                               // None -> Type error
        } } )*
}}
acc! {
    i32 => as_int / "expected int", i64 => as_long / "expected long",
    f64 => as_float / "expected float",
}

/// `tv!(Variant => Vec<ty> => err)` — pull a vector out by exact variant.
macro_rules! tv { ($($v:ident => $t:ty => $m:literal),* $(,)?) => {
    $( impl TryFrom<K> for $t { type Error = LError;
        fn try_from(k: K) -> Result<$t, LError> { match k {
            K::$v(x) => Ok(x), _ => Err(LError::Type($m.into())) } } } )*
}}
tv! { IntVec => Vec<i32> => "expected int vector",
      FloatVec => Vec<f64> => "expected float vector", }

impl TryFrom<K> for String {                                                    // CharVec or Symbol -> String
    type Error = LError;
    fn try_from(k: K) -> Result<String, LError> {
        match k {
            K::CharVec(v) => Ok(String::from_utf8_lossy(&v).into_owned()),
            K::Symbol(s)  => Ok(s),
            _ => Err(LError::Type("expected string or symbol".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn from_int()    { assert_eq!(K::from(42i32), K::Int(42)); }
    #[test] fn from_str()    { let k: K = "hello".into();
                               assert_eq!(k.as_string(), Some("hello")); }
    #[test] fn from_vec_i32(){ let k: K = vec![1i32, 2, 3].into();
                               assert_eq!(k, K::IntVec(vec![1, 2, 3])); }
    #[test] fn try_into_i32(){ let v: i32 = K::Int(42).try_into().unwrap();
                               assert_eq!(v, 42); }
}
