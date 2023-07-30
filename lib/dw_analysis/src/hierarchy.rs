//! Classes hierarchy graph representation.

use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::{Class, RepoCounters};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use petgraph::Direction;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use Direction::Outgoing;

#[derive(Debug, PartialEq, Eq)]
pub enum Inheritance {
    Extends,
    Implements,
}

impl fmt::Display for Inheritance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Extends => write!(f, "<extends>"),
            Self::Implements => write!(f, "<implements>"),
        }
    }
}

#[derive(Debug)]
pub struct Hierarchy<'a> {
    inner: DiGraph<Class<'a>, Inheritance>,
    node_ids: BTreeMap<String, NodeIndex>,
}

impl<'a> Hierarchy<'a> {
    pub(crate) fn new() -> Self {
        Self {
            inner: DiGraph::new(),
            node_ids: BTreeMap::new(),
        }
    }

    pub(crate) fn insert_class(&mut self, class: Class<'a>) -> AnalysisResult<()> {
        if self.node_ids.contains_key(class.name()) {
            return Err(AnalysisError::Internal(
                "duplicate object in hierarchy graph".to_string(),
            ));
        }

        let class_name = class.name().to_string();
        let id = self.inner.add_node(class);
        self.node_ids.insert(class_name, id);
        Ok(())
    }

    pub(crate) fn update_class(&mut self, class: Class<'a>) -> AnalysisResult<()> {
        if let Some(id) = self.node_ids.get(class.name()) {
            self.inner[*id] = class;
            Ok(())
        } else {
            Err(AnalysisError::ClassNotFound(class.name().to_string()))
        }
    }

    pub(crate) fn contains_class(&self, class_name: &str) -> bool {
        self.node_ids.contains_key(class_name)
    }

    pub fn iter_classes(&self) -> impl Iterator<Item = &Class> {
        self.inner.node_weights()
    }

    pub(crate) fn insert_extends(&mut self, class: &str, superclass: &str) -> AnalysisResult<()> {
        self.insert_link(class, superclass, Inheritance::Extends)
    }

    pub(crate) fn insert_implements(&mut self, class: &str, interface: &str) -> AnalysisResult<()> {
        self.insert_link(class, interface, Inheritance::Implements)
    }

    fn insert_link(&mut self, from: &str, to: &str, link: Inheritance) -> AnalysisResult<()> {
        let src = self
            .node_ids
            .get(from)
            .ok_or_else(|| AnalysisError::ClassNotFound(from.to_string()))?;
        let dst = self
            .node_ids
            .get(to)
            .ok_or_else(|| AnalysisError::ClassNotFound(to.to_string()))?;
        self.inner.add_edge(*src, *dst, link);
        Ok(())
    }

    pub(crate) fn close(&mut self, counters: &mut RepoCounters) {
        const JAVA_LANG_OBJECT: &str = "java/lang/Object";
        if !self.contains_class(JAVA_LANG_OBJECT) {
            self.insert_class(Class::new_no_def(
                counters.new_class_uid(),
                JAVA_LANG_OBJECT,
            ))
            .unwrap();
        }

        let id_orphans: Vec<NodeIndex> = self
            .inner
            .externals(Outgoing)
            .filter(|id| self.inner[*id].name() != JAVA_LANG_OBJECT)
            .collect();

        for id in id_orphans {
            let class = self.inner[id].clone();
            log::warn!(
                "add missing java.lang.Object inheritance to {}",
                class.name()
            );
            self.insert_extends(class.name(), JAVA_LANG_OBJECT).unwrap();
        }
    }

    #[must_use]
    pub fn get_class(&self, class_name: &str) -> Option<&Class> {
        self.node_ids.get(class_name).map(|id| &self.inner[*id])
    }

    #[must_use]
    pub fn all_parents(&self, class: &Class) -> BTreeSet<Class> {
        let id = self.node_ids.get(class.name()).unwrap();
        let mut parents = BTreeSet::new();
        let mut dfs = Dfs::new(&self.inner, *id);
        while let Some(id) = dfs.next(&self.inner) {
            parents.insert(self.inner[id].clone());
        }
        parents
    }

    #[must_use]
    pub fn to_dot(&self) -> String {
        format!(
            "{}",
            Dot::with_attr_getters(
                &self.inner,
                &[Config::EdgeNoLabel],
                &|_, edge| {
                    let style = match edge.weight() {
                        Inheritance::Extends => "solid",
                        Inheritance::Implements => "dashed",
                    };
                    format!("arrowType=empty,style={style}")
                },
                &|_, (_, class)| {
                    let (color, shape) = if class.is_defined() {
                        if class.is_system() {
                            ("#00000088", "box")
                        } else {
                            ("black", "box")
                        }
                    } else {
                        ("black", "none")
                    };
                    format!("color={color},shape={shape}")
                }
            )
        )
    }
}
