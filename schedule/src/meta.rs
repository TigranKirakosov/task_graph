use std::any::TypeId;

use crate::TaskMarker;

pub(crate) struct Meta {
    pub(crate) type_id: TypeId,
    #[cfg(any(test, feature = "visualizer"))]
    pub(crate) type_name: &'static str,
}

impl Meta {
    pub(crate) fn new<T: TaskMarker>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            #[cfg(any(test, feature = "visualizer"))]
            type_name: {
                let full_name = std::any::type_name::<T>();
                full_name
                    .rsplit_once("::")
                    .map(|(_, name)| name)
                    .unwrap_or(full_name)
            },
        }
    }
}
