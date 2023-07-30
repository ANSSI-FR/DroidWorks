use std::num::NonZeroUsize;

/// Unique id to identify a class in the repo
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct ClassUid(NonZeroUsize);

/// Unique id to identify a method in the repo
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct MethodUid(NonZeroUsize);

impl MethodUid {
    pub(crate) fn idx(self) -> usize {
        self.0.get() - 1
    }
}

/// Unique id to identify a field in the repo
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct FieldUid(NonZeroUsize);

impl FieldUid {
    pub(crate) fn idx(self) -> usize {
        self.0.get() - 1
    }
}

#[derive(Default)]
pub struct RepoCounters {
    nb_classes: usize,
    nb_methods: usize,
    nb_fields: usize,
}

impl RepoCounters {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn new_class_uid(&mut self) -> ClassUid {
        self.nb_classes += 1;
        ClassUid(NonZeroUsize::new(self.nb_classes).expect("just incremented, cannot be 0"))
    }

    pub(crate) fn new_method_uid(&mut self) -> MethodUid {
        self.nb_methods += 1;
        MethodUid(NonZeroUsize::new(self.nb_methods).expect("just incremented, cannot be 0"))
    }

    pub(crate) fn new_field_uid(&mut self) -> FieldUid {
        self.nb_fields += 1;
        FieldUid(NonZeroUsize::new(self.nb_fields).expect("just incremented, cannot be 0"))
    }

    pub(crate) fn nb_classes(&self) -> usize {
        self.nb_classes
    }

    pub(crate) fn nb_methods(&self) -> usize {
        self.nb_methods
    }

    pub(crate) fn nb_fields(&self) -> usize {
        self.nb_fields
    }
}
