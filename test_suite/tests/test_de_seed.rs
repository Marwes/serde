extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_derive_state;
extern crate serde_state;
extern crate serde_test;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use serde::de::{Deserialize, DeserializeSeed, Deserializer, Error};
use serde_state::de::DeserializeState;

use serde_test::{assert_de_seed_tokens, Token};

#[derive(Clone, Default)]
struct Seed(i32);

impl AsMut<Seed> for Seed {
    fn as_mut(&mut self) -> &mut Seed {
        self
    }
}

#[derive(Deserialize, Debug, PartialEq)]
struct Inner;

fn deserialize_inner<'de, S, D>(seed: &mut S, deserializer: D) -> Result<Inner, D::Error>
where
    S: AsMut<Seed>,
    D: Deserializer<'de>,
{
    Inner::deserialize_state(seed.as_mut(), deserializer)
}

impl<'de> DeserializeState<'de, Seed> for Inner {
    fn deserialize_state<D>(seed: &mut Seed, deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        seed.0 += 1;
        Inner::deserialize(deserializer)
    }
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "Seed")]
struct SeedStruct {
    #[serde(deserialize_state)]
    value: Inner,
    #[serde(deserialize_state_with = "deserialize_inner")]
    value2: Inner,
    value3: Inner,
}

#[test]
fn test_deserialize_state() {
    let value = SeedStruct {
        value: Inner,
        value2: Inner,
        value3: Inner,
    };
    let mut seed = Seed(0);
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::Struct {
                name: "SeedStruct",
                len: 3,
            },
            Token::Str("value"),
            Token::UnitStruct { name: "Inner" },
            Token::Str("value2"),
            Token::UnitStruct { name: "Inner" },
            Token::Str("value3"),
            Token::UnitStruct { name: "Inner" },
            Token::StructEnd,
        ],
    );

    assert_eq!(seed.0, 2);
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "Seed")]
struct Newtype(#[serde(deserialize_state_with = "deserialize_inner")] Inner);

#[test]
fn test_newtype_deserialize_state() {
    let value = Newtype(Inner);
    let mut seed = Seed::default();
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::NewtypeStruct { name: "Newtype" },
            Token::UnitStruct { name: "Inner" },
        ],
    );

    assert_eq!(seed.0, 1);
}

#[derive(Clone)]
struct ExtraParameterNewtypeSeed<T>(Seed, PhantomData<T>);

impl<T> AsMut<Seed> for ExtraParameterNewtypeSeed<T> {
    fn as_mut(&mut self) -> &mut Seed {
        &mut self.0
    }
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "ExtraParameterNewtypeSeed<T>")]
#[serde(de_parameters = "T")]
struct ExtraParameterNewtype(#[serde(deserialize_state_with = "deserialize_inner")] Inner);

#[test]
fn extra_parameter_test_newtype_deserialize_state() {
    let value = ExtraParameterNewtype(Inner);
    let mut seed = ExtraParameterNewtypeSeed(Seed::default(), PhantomData::<i32>);
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::NewtypeStruct {
                name: "ExtraParameterNewtype",
            },
            Token::UnitStruct { name: "Inner" },
        ],
    );

    assert_eq!((seed.0).0, 1);
}

#[derive(Clone)]
struct VecSeed<T>(T);

fn deserialize_vec<'de, T, U, D>(seed: &mut VecSeed<T>, deserializer: D) -> Result<Vec<U>, D::Error>
where
    D: Deserializer<'de>,
    U: DeserializeState<'de, T>,
{
    use serde_state::de::SeqSeedEx;
    deserializer.deserialize_seq(SeqSeedEx::new(&mut seed.0, Vec::with_capacity))
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "VecSeed<S>")]
#[serde(de_parameters = "S")]
#[serde(bound = "T: DeserializeState<'de, S>")]
struct VecNewtype<T>(#[serde(deserialize_state_with = "deserialize_vec")] Vec<T>);

#[test]
fn test_vec_newtype_deserialize_state() {
    let value = VecNewtype(vec![Inner, Inner]);
    let mut seed = VecSeed(Seed::default());
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::NewtypeStruct { name: "VecNewtype" },
            Token::Seq { len: Some(2) },
            Token::UnitStruct { name: "Inner" },
            Token::UnitStruct { name: "Inner" },
            Token::SeqEnd,
        ],
    );

    assert_eq!((seed.0).0, 2);
}

#[derive(Clone)]
struct GenericTypeSeed<T>(Seed, T);

impl<T> AsMut<Seed> for GenericTypeSeed<T> {
    fn as_mut(&mut self) -> &mut Seed {
        &mut self.0
    }
}

fn deserialize_nested_seed<'de, T, D>(
    seed: &mut GenericTypeSeed<T>,
    deserializer: D,
) -> Result<T::Value, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeSeed<'de> + Clone,
{
    seed.1.clone().deserialize(deserializer)
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "GenericTypeSeed<S>")]
#[serde(de_parameters = "S")]
#[serde(bound = "S: Clone + DeserializeSeed<'de, Value = T>")]
struct GenericType<T> {
    #[serde(deserialize_state_with = "deserialize_inner")]
    inner: Inner,
    #[serde(deserialize_state_with = "deserialize_nested_seed")]
    t: T,
}

#[test]
fn test_generic_deserialize_state() {
    let value = GenericType { inner: Inner, t: 3 };
    let mut seed = GenericTypeSeed(Seed::default(), PhantomData);
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::Struct {
                name: "GenericType",
                len: 2,
            },
            Token::String("inner"),
            Token::UnitStruct { name: "Inner" },
            Token::String("t"),
            Token::I32(3),
            Token::StructEnd,
        ],
    );

    assert_eq!((seed.0).0, 1);
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "Seed")]
enum Enum {
    Inner(#[serde(deserialize_state_with = "deserialize_inner")] Inner),
    Inner2(
        u32,
        #[serde(deserialize_state_with = "deserialize_inner")] Inner,
    ),
}

#[test]
fn test_enum_deserialize_state() {
    let value = Enum::Inner(Inner);
    let mut seed = Seed::default();
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::NewtypeVariant {
                name: "Enum",
                variant: "Inner",
            },
            Token::UnitStruct { name: "Inner" },
        ],
    );

    assert_eq!(seed.0, 1);
}

#[test]
fn test_enum_deserialize_state_2() {
    let value = Enum::Inner2(3, Inner);
    let mut seed = Seed::default();
    assert_de_seed_tokens(
        &mut seed,
        &value,
        &[
            Token::TupleVariant {
                name: "Enum",
                variant: "Inner2",
                len: 2,
            },
            Token::U32(3),
            Token::UnitStruct { name: "Inner" },
            Token::TupleVariantEnd,
        ],
    );

    assert_eq!(seed.0, 1);
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "NodeMap")]
struct Node {
    data: char,
    #[serde(deserialize_state_with = "deserialize_option_node")]
    left: Option<Rc<Node>>,
    #[serde(deserialize_state_with = "deserialize_option_node")]
    right: Option<Rc<Node>>,
}

fn deserialize_option_node<'de, D>(
    seed: &mut NodeMap,
    deserializer: D,
) -> Result<Option<Rc<Node>>, D::Error>
where
    D: Deserializer<'de>,
{
    let variant = Option::<Variant>::deserialize_state(seed, deserializer)?;
    match variant {
        None => Ok(None),
        Some(variant) => match variant {
            Variant::Marked {
                id,
                data,
                left,
                right,
            } => {
                let node = Rc::new(Node { data, left, right });
                seed.insert(id, Rc::clone(&node));
                Ok(Some(node))
            }
            Variant::Plain { data, left, right } => Ok(Some(Rc::new(Node { data, left, right }))),
            Variant::Reference(id) => match seed.get(&id) {
                Some(rc) => Ok(Some(Rc::clone(rc))),
                None => Err(Error::custom(format_args!("missing id {}", id))),
            },
        },
    }
}

type Id = u32;
type IdToShared<T> = HashMap<Id, T>;

type IdToNode = IdToShared<Rc<Node>>;

type NodeMap = IdToShared<Rc<Node>>;

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut map = IdToNode::default();
        Self::deserialize_state(&mut map, deserializer)
    }
}

//////////////////////////////////////////////////////////////////////////////

#[derive(DeserializeState)]
#[serde(deserialize_state = "NodeMap", rename = "Node")]
enum Variant {
    Plain {
        data: char,
        #[serde(deserialize_state_with = "deserialize_option_node")]
        left: Option<Rc<Node>>,
        #[serde(deserialize_state_with = "deserialize_option_node")]
        right: Option<Rc<Node>>,
    },
    Marked {
        id: u32,
        data: char,
        #[serde(deserialize_state_with = "deserialize_option_node")]
        left: Option<Rc<Node>>,
        #[serde(deserialize_state_with = "deserialize_option_node")]
        right: Option<Rc<Node>>,
    },
    Reference(u32),
}

#[test]
fn test_node_deserialize() {
    let b = Rc::new(Node {
        data: 'b',
        left: None,
        right: None,
    });
    let a = Rc::new(Node {
        data: 'a',
        left: Some(Rc::clone(&b)),
        right: Some(Rc::clone(&b)),
    });
    let map = HashMap::new();
    let mut seed = map;
    assert_de_seed_tokens(
        &mut seed,
        &*a,
        &[
            Token::Struct {
                name: "Node",
                len: 3,
            },
            Token::Str("data"),
            Token::Char('a'),
            Token::Str("left"),
            Token::Some,
            Token::StructVariant {
                name: "Node",
                variant: "Marked",
                len: 4,
            },
            Token::Str("id"),
            Token::I32(0),
            Token::Str("data"),
            Token::Char('b'),
            Token::Str("left"),
            Token::None,
            Token::Str("right"),
            Token::None,
            Token::StructVariantEnd,
            Token::Str("right"),
            Token::Some,
            Token::NewtypeVariant {
                name: "Node",
                variant: "Reference",
            },
            Token::U32(0),
            Token::StructEnd,
        ],
    );
}

#[derive(DeserializeState, Debug, PartialEq)]
#[serde(deserialize_state = "Seed")]
struct Attr {
    #[serde(with = "i32")]
    x: i32,
}
