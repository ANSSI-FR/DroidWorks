use crate::errors::AnalysisResult;
use crate::repo::*;
use crate::stats::is_stub;
use dw_dex::classes::{ClassDefItem, ClassFlags};
use dw_dex::types::Type;
use dw_dex::{Dex, DexCollection, DexIndex, Index};
use regex::Regex;
use std::cmp::Ordering;
use std::fmt;

/// The enriched class definition.
#[derive(Debug, Clone)]
pub struct Class<'a> {
    // Unique identifier in the repository
    uid: ClassUid,
    // Dex optional class data and corresponding dex reference to resolve data
    dex_content: Option<(Index<ClassDefItem>, &'a Dex)>,
    // Flag to indicate that the class is part of the API and part of the analyzed application
    system: bool,
    // Cache of name that identify the class
    name: String,
    // List of contained methods (declaration level)
    methods: Vec<MethodUid>,
    // List of contained fields (declaration level)
    fields: Vec<FieldUid>,
}

impl<'a> PartialEq for Class<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

impl<'a> Eq for Class<'a> {}

impl<'a> PartialOrd for Class<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.uid.partial_cmp(&other.uid)
    }
}

impl<'a> Ord for Class<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.uid.cmp(&other.uid)
    }
}

impl<'a> fmt::Display for Class<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'a> Class<'a> {
    fn new(
        class_uid: ClassUid,
        content: Index<ClassDefItem>,
        dex: &'a Dex,
        system: bool,
        counters: &mut RepoCounters,
        methods: &mut Vec<Method<'a>>,
        fields: &mut Vec<Field<'a>>,
    ) -> AnalysisResult<Self> {
        let class_def = content.get(dex)?;
        let name = class_def.class_name(dex)?;
        let mut class_methods = Vec::new();
        let mut class_fields = Vec::new();
        if let Some(data) = class_def.data(dex)? {
            for encoded_method in data.iter_methods() {
                let method_uid = counters.new_method_uid();
                let new_method = Method::new(method_uid, encoded_method, dex)?;
                methods.push(new_method);
                class_methods.push(method_uid);
            }
            for encoded_field in data.iter_fields() {
                let field_uid = counters.new_field_uid();
                let new_field = Field::new(field_uid, encoded_field, dex)?;
                fields.push(new_field);
                class_fields.push(field_uid);
            }
        }

        Ok(Self {
            uid: class_uid,
            dex_content: Some((content, dex)),
            system,
            name,
            methods: class_methods,
            fields: class_fields,
        })
    }

    /// Builds an enriched class definition with implementation from a raw Dex definition.
    pub(crate) fn new_impl(
        class_uid: ClassUid,
        class_def: &'a ClassDefItem,
        dex: &'a Dex,
        counters: &mut RepoCounters,
        methods: &mut Vec<Method<'a>>,
        fields: &mut Vec<Field<'a>>,
    ) -> AnalysisResult<Self> {
        Self::new(
            class_uid,
            class_def.index(),
            dex,
            false,
            counters,
            methods,
            fields,
        )
    }

    /// Builds an enriched system class definition from a raw Dex definition.
    pub(crate) fn new_sys(
        class_uid: ClassUid,
        class_def: &'a ClassDefItem,
        dex: &'a Dex,
        counters: &mut RepoCounters,
        methods: &mut Vec<Method<'a>>,
        fields: &mut Vec<Field<'a>>,
    ) -> AnalysisResult<Self> {
        Self::new(
            class_uid,
            class_def.index(),
            dex,
            true,
            counters,
            methods,
            fields,
        )
    }

    /// Builds an enriched class declaration from a raw Dex definition.
    pub(crate) fn new_no_def(class_uid: ClassUid, name: &str) -> Self {
        Self {
            uid: class_uid,
            dex_content: None,
            system: false,
            name: name.to_string(),
            methods: Vec::new(),
            fields: Vec::new(),
        }
    }

    #[inline]
    pub fn uid(&self) -> ClassUid {
        self.uid
    }

    fn content(&self) -> Option<&ClassDefItem> {
        self.dex_content
            .map(|(content, dex)| content.get(dex).expect("invalid dex ref"))
    }

    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    #[must_use]
    pub const fn is_defined(&self) -> bool {
        self.dex_content.is_some()
    }

    #[inline]
    #[must_use]
    pub const fn is_system(&self) -> bool {
        self.system
    }

    /// Estimate if the class contains stub. Heuristics rely on the presence
    /// of the 'Stub!' string and on the instruction counts (small methods).
    pub fn has_stub_code(&self, repo: &'a Repo) -> AnalysisResult<bool> {
        let Some((_, dex)) = self.dex_content else {
            return Ok(false)
        };
        for method in self.iter_methods(repo).filter(|m| m.code().is_some()) {
            let code = method.code().unwrap();
            if is_stub(&code.read().unwrap(), dex)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Returns a iterator over all methods contained in the class.
    pub fn iter_methods(&self, repo: &'a Repo) -> impl Iterator<Item = &Method> {
        self.methods.iter().map(|muid| &repo[*muid])
    }

    pub fn get_method(
        &self,
        name: &str,
        return_type: &Type,
        parameters_types: &[Type],
        repo: &'a Repo,
    ) -> Option<&Method> {
        self.iter_methods(repo).find(|meth| {
            meth.name() == name
                && meth.return_type() == return_type
                && meth.parameters_types() == parameters_types
        })
    }

    pub fn find_methods(
        &'a self,
        pattern: &'a Regex,
        repo: &'a Repo,
    ) -> impl Iterator<Item = &'a Method> {
        self.iter_methods(repo)
            .filter(|m| pattern.is_match(m.name()))
    }

    pub fn iter_fields(&self, repo: &'a Repo) -> impl Iterator<Item = &Field> {
        self.fields.iter().map(|fuid| &repo[*fuid])
    }

    pub fn get_field(&self, name: &str, type_: &Type, repo: &'a Repo) -> Option<&Field> {
        self.iter_fields(repo)
            .find(|field| field.name() == name && field.type_() == type_)
    }

    pub fn find_fields(
        &'a self,
        pattern: &'a Regex,
        repo: &'a Repo,
    ) -> impl Iterator<Item = &'a Field> {
        self.iter_fields(repo)
            .filter(|f| pattern.is_match(f.name()))
    }

    #[inline]
    #[must_use]
    pub fn is_public(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_PUBLIC)
    }

    #[inline]
    #[must_use]
    pub fn is_private(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_PRIVATE)
    }

    #[inline]
    #[must_use]
    pub fn is_protected(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_PROTECTED)
    }

    #[inline]
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_STATIC)
    }

    #[inline]
    #[must_use]
    pub fn is_final(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_FINAL)
    }

    #[inline]
    #[must_use]
    pub fn is_interface(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_INTERFACE)
    }

    #[inline]
    #[must_use]
    pub fn is_abstract(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_ABSTRACT)
    }

    #[inline]
    #[must_use]
    pub fn is_synthetic(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_SYNTHETIC)
    }

    #[inline]
    #[must_use]
    pub fn is_annotation(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_ANNOTATION)
    }

    #[inline]
    #[must_use]
    pub fn is_enum(&self) -> bool {
        self.content()
            .expect("content")
            .flags()
            .contains(ClassFlags::ACC_ENUM)
    }
}
