//! A repository to centralize application and dependencies classes.

use crate::callgraph::CallGraph;
use crate::errors::{AnalysisError, AnalysisResult};
use crate::hierarchy::Hierarchy;
use crate::repo::*;
use dw_dex::classes::ClassDefItem;
use dw_dex::Dex;
use regex::Regex;
use std::ops;

pub struct Repo<'a> {
    dexs: Vec<&'a Dex>,
    hierarchy: Hierarchy<'a>,
    counters: RepoCounters,
    methods: Vec<Method<'a>>,
    fields: Vec<Field<'a>>,
}

impl<'a> Default for Repo<'a> {
    fn default() -> Self {
        Self {
            dexs: Vec::new(),
            hierarchy: Hierarchy::new(),
            counters: RepoCounters::new(),
            methods: Vec::new(),
            fields: Vec::new(),
        }
    }
}

impl<'a> ops::Index<MethodUid> for Repo<'a> {
    type Output = Method<'a>;

    fn index(&self, muid: MethodUid) -> &Method<'a> {
        &self.methods[muid.idx()]
    }
}

impl<'a> ops::Index<FieldUid> for Repo<'a> {
    type Output = Field<'a>;

    fn index(&self, fuid: FieldUid) -> &Field<'a> {
        &self.fields[fuid.idx()]
    }
}

impl<'a> Repo<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_dex(&mut self, dex: &'a Dex, is_system: bool) -> AnalysisResult<()> {
        self.dexs.push(dex);

        for class_def in dex.iter_class_defs() {
            self.register_class(class_def, dex, is_system)?;
        }

        Ok(())
    }

    fn register_class(
        &mut self,
        class_def: &'a ClassDefItem,
        dex: &'a Dex,
        is_system: bool,
    ) -> AnalysisResult<()> {
        let class_name = class_def.class_name(dex)?;
        log::trace!(
            "pushing '{}'{} in repository",
            class_name,
            if is_system { " (SYS)" } else { "" }
        );

        let mut uid_to_update = None;
        if let Some(class_h) = self.hierarchy.get_class(&class_name) {
            if class_h.is_defined() {
                log::warn!(
                    "class '{}'{} has already been pushed in repository",
                    class_name,
                    if is_system { " (SYS)" } else { "" }
                );
                // no change of the hierarchy nor of the repository for this class
                return Ok(());
            }
            uid_to_update = Some(class_h.uid());
        }

        let class = if is_system {
            Class::new_sys(
                uid_to_update.unwrap_or_else(|| self.counters.new_class_uid()),
                class_def,
                dex,
                &mut self.counters,
                &mut self.methods,
                &mut self.fields,
            )
        } else {
            Class::new_impl(
                uid_to_update.unwrap_or_else(|| self.counters.new_class_uid()),
                class_def,
                dex,
                &mut self.counters,
                &mut self.methods,
                &mut self.fields,
            )
        }?;
        if uid_to_update.is_some() {
            self.hierarchy.update_class(class)?;
        } else {
            self.hierarchy.insert_class(class)?;
        }

        // filling in the hierarchy links
        if let Some(superclass_name) = class_def.superclass(dex)? {
            if !self.hierarchy.contains_class(&superclass_name) {
                self.hierarchy.insert_class(Class::new_no_def(
                    self.counters.new_class_uid(),
                    &superclass_name,
                ))?;
            }
            self.hierarchy
                .insert_extends(&class_name, &superclass_name)?;
        }
        for interface_name in &class_def.interfaces(dex)? {
            if !self.hierarchy.contains_class(interface_name) {
                self.hierarchy.insert_class(Class::new_no_def(
                    self.counters.new_class_uid(),
                    interface_name,
                ))?;
            }
            self.hierarchy
                .insert_implements(&class_name, interface_name)?;
        }

        Ok(())
    }

    pub fn close_hierarchy(&mut self) {
        self.hierarchy.close(&mut self.counters);
    }

    #[inline]
    #[must_use]
    pub const fn hierarchy(&self) -> &Hierarchy {
        &self.hierarchy
    }

    #[inline]
    pub fn iter_classes(&self) -> impl Iterator<Item = &Class> {
        self.hierarchy.iter_classes()
    }

    pub fn iter_missing_classes(&self) -> impl Iterator<Item = &str> {
        self.hierarchy
            .iter_classes()
            .filter_map(|class| (!class.is_defined()).then(|| class.name()))
    }

    pub fn get_class_by_name(&self, name: &str) -> Option<&Class> {
        self.hierarchy.get_class(name)
    }

    pub fn find_classes(&'a self, pattern: &'a Regex) -> impl Iterator<Item = &'a Class> {
        self.hierarchy
            .iter_classes()
            .filter(|class| pattern.is_match(class.name()))
    }

    pub fn iter_classes_methods(&self) -> impl Iterator<Item = (&Class, &Method)> {
        self.iter_classes()
            .flat_map(move |class| class.iter_methods(self).map(move |method| (class, method)))
    }

    pub(crate) fn find_method_by_descriptor(&self, descriptor: &MethodDescr) -> Option<&Method> {
        let class = self.get_class_by_name(&descriptor.definer().class_name())?;
        class.get_method(
            descriptor.name(),
            descriptor.return_type(),
            descriptor.parameters_types(),
            self,
        )
    }

    pub(crate) fn is_inherited(&self, descriptor: &MethodDescr) -> bool {
        let Some(class) = self.get_class_by_name(&descriptor.definer().class_name()) else { return false };
        for parent in self.hierarchy.all_parents(class) {
            if parent
                .get_method(
                    descriptor.name(),
                    descriptor.return_type(),
                    descriptor.parameters_types(),
                    self,
                )
                .is_some()
            {
                return true;
            }
        }
        false
    }

    pub fn build_callgraph(&self) -> AnalysisResult<CallGraph> {
        CallGraph::build(self, false)
    }

    /// Checks whether an object can be replaced by another with a typechecking point of view.
    /// The only accepted types are class names here.
    ///
    /// This method returns `true` if one of the following case occurs:
    ///  - `type_name1` and `type_name2` are the same (in this case, there is no check that
    /// `type_name1` and `type_name2` exist in the class hierarchy),
    ///  - `type_name2` is `java/lang/Object`. This holds because every class must directly or
    /// indirectly inherit from Object, thus its a subtype of it (in this case, there is no
    /// check that `type_name1` exists in the class hierarchy, nor `type_name2` although
    /// `java/lang/Object` is likely to be the root of this hierarchy)),
    /// - there exists a inheritance path leeding from `type_name2` to `type_name1`.
    pub fn is_typeable_as(&self, type_name1: &str, type_name2: &str) -> AnalysisResult<bool> {
        // these optimization allows to handle cases where classes are not known in the repository
        // (i.e. no system/sdk given).

        // every type is typeable as itself:
        if type_name1 == type_name2 {
            return Ok(true);
        }

        // every class inherits (directly or indirectly) from java.lang.Object:
        if type_name2 == "java/lang/Object" {
            return Ok(true);
        }

        let class1 = self
            .hierarchy
            .get_class(type_name1)
            .ok_or_else(|| AnalysisError::ClassNotFound(type_name1.to_string()))?;
        let class2 = self
            .hierarchy
            .get_class(type_name2)
            .ok_or_else(|| AnalysisError::ClassNotFound(type_name2.to_string()))?;

        Ok(self
            .hierarchy
            .all_parents(class1)
            .iter()
            .any(|parent| parent == class2))
    }

    /// Return a list of classes (or interfaces) names that represent the least
    /// common types between two objects in the class hierarchy.
    ///
    /// There exists some cases (i.e. `type_name1` and `type_name2` are the same object)
    /// for which there is no check that `type_name1` or `type_name2` actually exist in the
    /// class hierarchy.
    pub fn least_common_types(
        &self,
        type_name1: &str,
        type_name2: &str,
    ) -> AnalysisResult<Vec<String>> {
        // handle trivial cases
        if self.is_typeable_as(type_name1, type_name2)? {
            Ok(vec![type_name2.to_string()])
        } else if self.is_typeable_as(type_name2, type_name1)? {
            Ok(vec![type_name1.to_string()])
        } else {
            // retrieve class nodes in hierarchy graph
            let class1 = self
                .hierarchy
                .get_class(type_name1)
                .ok_or_else(|| AnalysisError::ClassNotFound(type_name1.to_string()))?;
            let class2 = self
                .hierarchy
                .get_class(type_name2)
                .ok_or_else(|| AnalysisError::ClassNotFound(type_name2.to_string()))?;

            // compute common super-types set
            let c1_types = self.hierarchy.all_parents(class1);
            let c2_types = self.hierarchy.all_parents(class2);
            let mut common = c1_types.intersection(&c2_types).collect::<Vec<_>>();
            if common.is_empty() {
                log::error!("will fail because of empty join result of object types");
                log::error!("\tc1_types:");
                for t in &c1_types {
                    log::error!("\t\t{}", t.name());
                }
                log::error!("\tc2_types:");
                for t in &c2_types {
                    log::error!("\t\t{}", t.name());
                }
                common = vec![self.hierarchy.get_class("java/lang/Object").unwrap()];
            }
            assert!(!common.is_empty());

            // filter to keep only least types
            let mut res = Vec::new();
            for obj in &common {
                let mut keep_it = false;
                for other in &common {
                    if (other.name() == obj.name())
                        || !self.is_typeable_as(other.name(), obj.name())?
                    {
                        keep_it = true;
                        break;
                    }
                }
                if keep_it {
                    res.push(obj.name().to_string());
                }
            }
            Ok(res)
        }
    }

    pub fn nb_classes(&self) -> usize {
        self.counters.nb_classes()
    }

    pub fn nb_methods(&self) -> usize {
        self.counters.nb_methods()
    }

    pub fn nb_fields(&self) -> usize {
        self.counters.nb_fields()
    }
}
