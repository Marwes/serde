// Copyright 2017 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # serde_seed
//!
//! `serde_seed` is a crate which extends the normal `Deserialize` and `Serialize` traits to allow
//! state to be passed to every value which is serialized or deserialized.
//!
//! ## Example
//!
//! ```
//! extern crate serde_json;
//! extern crate serde_seed as serde;
//! #[macro_use]
//! extern crate serde_derive;
//! #[macro_use]
//! extern crate serde_derive_seed;
//!
//! use std::borrow::BorrowMut;
//! use std::cell::Cell;
//! use serde::ser::{Serialize, Serializer, SerializeSeed};
//! use serde::de::{Deserialize, Deserializer, DeserializeSeedEx};
//!
//! #[derive(Deserialize, Serialize)]
//! struct Inner;
//!
//! impl SerializeSeed for Inner {
//!     type Seed = Cell<i32>;
//! 
//!     fn serialize_seed<S>(&self, serializer: S, seed: &Self::Seed) -> Result<S::Ok, S::Error>
//!     where
//!         S: Serializer,
//!     {
//!         seed.set(seed.get() + 1);
//!         self.serialize(serializer)
//!     }
//! }
//!
//! impl<'de, S> DeserializeSeedEx<'de, S> for Inner where S: BorrowMut<i32> {
//! 
//!     fn deserialize_seed<D>(seed: &mut S, deserializer: D) -> Result<Self, D::Error>
//!     where
//!         D: Deserializer<'de>,
//!     {
//!         *seed.borrow_mut() += 1;
//!         Self::deserialize(deserializer)
//!     }
//! }
//!
//! #[derive(SerializeSeed, DeserializeSeed)]
//!
//! // `serialize_seed` or `deserialize_seed` is necessary to tell the derived implementation which
//! // seed that is passed
//! #[serde(serialize_seed = "Cell<i32>")]
//!
//! // `de_parameters` can be used to specify additional type parameters for the derived instance
//! #[serde(de_parameters = "S")]
//! #[serde(bound(deserialize = "S: BorrowMut<i32>"))]
//! #[serde(deserialize_seed = "S")]
//! struct Struct {
//!     // The `serialize_seed` attribute must be specified to use seeded serialization
//!     #[serde(serialize_seed)]
//!     // The `deserialize_seed` attribute must be specified to use seeded deserialization
//!     #[serde(deserialize_seed)]
//!     value: Inner,
//!
//!     // The `seed` attribute can be used to specify `deserialize_seed` and `serialize_seed`
//!     // simultaneously
//!     #[serde(seed)]
//!     value2: Inner,
//!
//!     // If no attributes are specified then normal serialization and/or deserialization is used
//!     value3: Inner,
//!
//!     // The `[de]serialize_seed_with` attribute can be used to specify a custom function which
//!     // does the serialization or deserialization
//!     #[serde(serialize_seed_with = "serialize_inner")]
//!     value4: Inner,
//! }
//!
//! fn serialize_inner<S>(self_: &Inner, serializer: S, seed: &Cell<i32>) -> Result<S::Ok, S::Error>
//!     where S: Serializer
//! {
//!     seed.set(seed.get() + 10);
//!     self_.serialize(serializer)
//! }
//!
//! fn main() {
//!     let s = Struct {
//!         value: Inner,
//!         value2: Inner,
//!         value3: Inner,
//!         value4: Inner,
//!     };
//!
//!     let mut buffer = Vec::new();
//!     {
//!         let mut serializer = serde_json::Serializer::pretty(&mut buffer);
//!         let seed = Cell::new(0);
//!         s.serialize_seed(&mut serializer, &seed).unwrap();
//!         assert_eq!(seed.get(), 12);
//!     }
//!     {
//!         let mut deserializer = serde_json::Deserializer::from_slice(&buffer);
//!         let mut seed = 0;
//!         Struct::deserialize_seed(&mut seed, &mut deserializer).unwrap();
//!         assert_eq!(seed, 2);
//!     }
//! }
//!
//! ```

////////////////////////////////////////////////////////////////////////////////

// Serde types in rustdoc of other crates get linked to here.
#![doc(html_root_url = "https://docs.rs/serde/1.0.8")]

// Support using Serde without the standard library!
#![cfg_attr(not(feature = "std"), no_std)]

// Unstable functionality only if the user asks for it. For tracking and
// discussion of these features please refer to this issue:
//
//    https://github.com/serde-rs/serde/issues/812
#![cfg_attr(feature = "unstable", feature(nonzero, specialization))]
#![cfg_attr(all(feature = "std", feature = "unstable"), feature(into_boxed_c_str))]
#![cfg_attr(feature = "alloc", feature(alloc))]
#![cfg_attr(feature = "collections", feature(collections))]

// Whitelisted clippy lints.
#![cfg_attr(feature = "cargo-clippy", allow(doc_markdown))]
#![cfg_attr(feature = "cargo-clippy", allow(linkedlist))]
#![cfg_attr(feature = "cargo-clippy", allow(type_complexity))]
#![cfg_attr(feature = "cargo-clippy", allow(zero_prefixed_literal))]

// Blacklisted Rust lints.
#![deny(missing_docs, unused_imports)]

////////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "collections")]
extern crate collections;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "unstable", feature = "std"))]
extern crate core;

#[macro_use]
extern crate serde;

/// A facade around all the types we need from the `std`, `core`, `alloc`, and
/// `collections` crates. This avoids elaborate import wrangling having to
/// happen in every module.
mod lib {
    mod core {
        #[cfg(feature = "std")]
        pub use std::*;
        #[cfg(not(feature = "std"))]
        pub use core::*;
    }

    pub use self::core::{cmp, iter, mem, ops, slice, str};
    pub use self::core::{i8, i16, i32, i64, isize};
    pub use self::core::{u8, u16, u32, u64, usize};
    pub use self::core::{f32, f64};

    pub use self::core::cell::{Cell, RefCell};
    pub use self::core::clone::{self, Clone};
    pub use self::core::convert::{self, From, Into};
    pub use self::core::default::{self, Default};
    pub use self::core::fmt::{self, Debug, Display};
    pub use self::core::marker::{self, PhantomData};
    pub use self::core::option::{self, Option};
    pub use self::core::result::{self, Result};

    #[cfg(feature = "std")]
    pub use std::borrow::{Cow, ToOwned};
    #[cfg(all(feature = "collections", not(feature = "std")))]
    pub use collections::borrow::{Cow, ToOwned};

    #[cfg(feature = "std")]
    pub use std::string::String;
    #[cfg(all(feature = "collections", not(feature = "std")))]
    pub use collections::string::{String, ToString};

    #[cfg(feature = "std")]
    pub use std::vec::Vec;
    #[cfg(all(feature = "collections", not(feature = "std")))]
    pub use collections::vec::Vec;

    #[cfg(feature = "std")]
    pub use std::boxed::Box;
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::boxed::Box;

    #[cfg(all(feature = "rc", feature = "std"))]
    pub use std::rc::Rc;
    #[cfg(all(feature = "rc", feature = "alloc", not(feature = "std")))]
    pub use alloc::rc::Rc;

    #[cfg(all(feature = "rc", feature = "std"))]
    pub use std::sync::Arc;
    #[cfg(all(feature = "rc", feature = "alloc", not(feature = "std")))]
    pub use alloc::arc::Arc;

    #[cfg(feature = "std")]
    pub use std::collections::{BinaryHeap, BTreeMap, BTreeSet, LinkedList, VecDeque};
    #[cfg(all(feature = "collections", not(feature = "std")))]
    pub use collections::{BinaryHeap, BTreeMap, BTreeSet, LinkedList, VecDeque};

    #[cfg(feature = "std")]
    pub use std::{error, net};

    #[cfg(feature = "std")]
    pub use std::collections::{HashMap, HashSet};
    #[cfg(feature = "std")]
    pub use std::ffi::{CString, CStr, OsString, OsStr};
    #[cfg(feature = "std")]
    pub use std::hash::{Hash, BuildHasher};
    #[cfg(feature = "std")]
    pub use std::io::Write;
    #[cfg(feature = "std")]
    pub use std::path::{Path, PathBuf};
    #[cfg(feature = "std")]
    pub use std::time::Duration;
    #[cfg(feature = "std")]
    pub use std::sync::{Mutex, RwLock};

    #[cfg(feature = "unstable")]
    pub use core::nonzero::{NonZero, Zeroable};
}

////////////////////////////////////////////////////////////////////////////////

pub mod ser;
pub mod de;

#[doc(hidden)]
pub mod private;

pub use serde::*;
