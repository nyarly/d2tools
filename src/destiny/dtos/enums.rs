use std::fmt;
use uritemplate::{TemplateVar, IntoTemplateVar};

macro_rules! enum_number {
    ($name:ident { $($variant:ident = $value:expr, )* }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name {
            $($variant = $value,)*
        }

        impl IntoTemplateVar for $name {
          fn into_template_var(self) -> TemplateVar {
            let int = self as i32;
            TemplateVar::Scalar(int.to_string())
          }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: ::serde::Serializer
            {
                // Serialize the enum as a i32.
                serializer.serialize_i32(*self as i32)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: ::serde::Deserializer<'de>
            {
                struct Visitor;

                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a signed integer")
                    }

                    fn visit_i64<E>(self, value: i64) -> Result<$name, E>
                        where E: ::serde::de::Error
                    {
                        // Rust does not come with a simple way of converting a
                        // number to an enum, so use a big `match`.
                        match value {
                            $( $value => Ok($name::$variant), )*
                            _ => Err(E::custom(
                                format!("unknown {} value: {}",
                                stringify!($name), value))),
                        }
                    }

                    fn visit_i32<E>(self, value: i32) -> Result<$name, E>
                        where E: ::serde::de::Error
                    {
                      self.visit_i64(value as i64)
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<$name, E>
                        where E: ::serde::de::Error
                    {
                      self.visit_i64(value as i64)
                    }

                    fn visit_u32<E>(self, value: u32) -> Result<$name, E>
                        where E: ::serde::de::Error
                    {
                      self.visit_u64(value as u64)
                    }

                }

                // Deserialize the enum from a i32.
                deserializer.deserialize_i32(Visitor)
            }
        }
    }
}

enum_number!(BungieMemberType {
  TigerXbox = 1,
  TigerPsn = 2,
  TigerBlizzard = 4,
  TigerDemon = 10,
  BungieNext = 254,
  All = -1,
});

enum_number!(ComponentType {
  None = 0,
  Profiles = 100,
  VendorReceipts = 101,
  ProfileInventories = 102,
  ProfileCurrencies = 103,
  Characters = 200,
  CharacterInventories = 201,
  CharacterProgressions = 202,
  CharacterRenderData = 203,
  CharacterActivities = 204,
  CharacterEquipment = 205,
  ItemInstances = 300,
  ItemObjectives = 301,
  ItemPerks = 302,
  ItemRenderData = 303,
  ItemStats = 304,
  ItemSockets = 305,
  ItemTalentGrids = 306,
  ItemCommonData = 307,
  ItemPlugStates = 308,
  Vendors = 400,
  VendorCategories = 401,
  VendorSales = 402,
  Kiosks = 500,
});


pub fn component_list<'g, T, U>(t: T) -> TemplateVar
  where T: IntoIterator<Item = &'g ComponentType, IntoIter = U>,
        U: Iterator<Item = &'g ComponentType> + Sized
{
  TemplateVar::List(t.into_iter().map(|el| (*el as i32).to_string()).collect())
}
