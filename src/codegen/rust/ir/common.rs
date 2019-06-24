pub struct Mods {}

#[allow(non_snake_case)]
impl Mods {
    pub fn lavish() -> String {
        "::lavish".into()
    }

    pub fn io() -> String {
        "::std::io".into()
    }

    pub fn facts() -> String {
        format!("{lavish}::facts", lavish = Self::lavish())
    }

    pub fn chrono() -> String {
        format!("{lavish}::chrono", lavish = Self::lavish())
    }

    pub fn collections() -> String {
        "::std::collections".into()
    }

    pub fn sync() -> String {
        "::std::sync".into()
    }

    pub fn es() -> String {
        format!("{}::erased_serde", Self::lavish())
    }

    pub fn serde() -> String {
        format!("{}::serde", Self::lavish())
    }

    pub fn serde_derive() -> String {
        format!("{}::serde_derive", Self::lavish())
    }
}

pub struct Traits {}

#[allow(non_snake_case)]
impl Traits {
    pub fn Serialize() -> String {
        format!("{}::Serialize", Mods::serde_derive())
    }

    pub fn Deserialize() -> String {
        format!("{}::Deserialize", Mods::serde_derive())
    }

    pub fn Factual() -> String {
        format!("{}::Factual", Mods::facts())
    }

    pub fn Atom() -> String {
        format!("{}::Atom", Mods::lavish())
    }

    pub fn Read() -> String {
        format!("{}::Read", Mods::io())
    }

    pub fn Write() -> String {
        format!("{}::Write", Mods::io())
    }
}

pub struct Structs {}

#[allow(non_snake_case)]
impl Structs {
    pub fn Deserializer() -> String {
        format!("{}::Deserializer", Mods::es())
    }

    pub fn Error() -> String {
        format!("{}::Error", Mods::lavish())
    }

    pub fn FactsError() -> String {
        format!("{}::Error", Mods::facts())
    }

    pub fn Arc() -> String {
        format!("{}::Arc", Mods::sync())
    }

    pub fn HashMap() -> String {
        format!("{}::HashMap", Mods::collections())
    }
}
