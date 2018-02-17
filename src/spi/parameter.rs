use types::{oid, StaticallyTyped};
use Datum;

pub struct Parameter<'a> {
    pub(super) oid: oid,
    pub(super) value: Option<Datum<'a>>,
}

impl<'a> Parameter<'a> {
    pub fn null<T: StaticallyTyped>() -> Parameter<'a> {
        Parameter {
            oid: T::OID,
            value: None,
        }
    }
}

impl<'a, T: StaticallyTyped + Into<Datum<'a>>> From<T> for Parameter<'a> {
    fn from(t: T) -> Parameter<'a> {
        Parameter {
            oid: T::OID,
            value: Some(t.into()),
        }
    }
}
