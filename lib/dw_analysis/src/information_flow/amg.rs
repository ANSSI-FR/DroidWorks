use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Write},
    hash::Hash,
};

use dw_dex::Addr;
use fasthash::HasherExt;

use crate::{
    information_flow::errors::FlowError,
    repo::{ClassUid, FieldUid, MethodUid, Repo},
};

use super::errors::FlowResult;

#[derive(Debug, Clone)]
pub(crate) struct Amg {
    graph: petgraph::graphmap::DiGraphMap<VertexHash, EdgeWeight>,
    vertex: BTreeMap<VertexHash, Vertex>,
}

impl PartialEq for Amg {
    fn eq(&self, other: &Self) -> bool {
        petgraph::algo::is_isomorphic(&self.graph, &other.graph)
    }
}

impl Eq for Amg {}

impl Default for Amg {
    fn default() -> Self {
        let graph = petgraph::graphmap::DiGraphMap::default();
        Self {
            graph,
            vertex: BTreeMap::default(),
        }
    }
}

impl Amg {
    pub(crate) fn add_vertex(&mut self, vertex: Vertex) -> VertexHash {
        let mut hasher = fasthash::Murmur3HasherExt::default();
        vertex.hash(&mut hasher);
        let vhash = VertexHash(hasher.finish_ext());
        if !self.graph.contains_node(vhash) {
            self.graph.add_node(vhash);
            self.vertex.insert(vhash, vertex);
        }
        vhash
    }

    pub(crate) fn vertex(&self, vh: VertexHash) -> &Vertex {
        &self.vertex[&vh]
    }

    pub(crate) fn add_edge(&mut self, from_vertex: VertexHash, to_vertex: VertexHash, edge: Edge) {
        match self.graph.edge_weight_mut(from_vertex, to_vertex) {
            None => {
                self.graph
                    .add_edge(from_vertex, to_vertex, EdgeWeight::from(edge));
            }
            Some(edgew) => edgew.insert(edge),
        }
    }

    pub(crate) fn successors(&self, vertex: VertexHash) -> BTreeSet<(&EdgeWeight, VertexHash)> {
        self.graph
            .edges_directed(vertex, petgraph::Direction::Outgoing)
            .map(|(_, n, e)| (e, n))
            .collect()
    }

    pub(crate) fn join(&mut self, other: &Self) {
        for (ovhash, overtex) in &other.vertex {
            if !self.vertex.contains_key(ovhash) {
                self.graph.add_node(*ovhash);
                self.vertex.insert(*ovhash, overtex.clone());
            }
        }

        for (ofrom, oto, oedgew) in other.graph.all_edges() {
            match self.graph.edge_weight_mut(ofrom, oto) {
                None => {
                    self.graph.add_edge(ofrom, oto, oedgew.clone());
                }
                Some(edgew) => edgew.join(oedgew),
            }
        }
    }

    pub(crate) fn inject(
        &mut self,
        other: &Self,
        parameter_flows: Vec<Flows>,
    ) -> FlowResult<BTreeMap<VertexHash, BTreeSet<VertexHash>>> {
        let mut mapping = BTreeMap::new();

        for (ovhash, overtex) in &other.vertex {
            if !self.graph.contains_node(*ovhash) {
                match overtex {
                    Vertex::Parameter(_, param, fields) => {
                        for pflow in &parameter_flows[*param] {
                            let pflow_vertex = &self.vertex[&pflow.vertex_hash()];
                            let pflow_fields_vertex = pflow_vertex.fields(fields)?;
                            let pflow_fields_hash = self.add_vertex(pflow_fields_vertex);
                            match mapping.entry(*ovhash) {
                                std::collections::btree_map::Entry::Vacant(e) => {
                                    let mut m = BTreeSet::new();
                                    m.insert(pflow_fields_hash);
                                    e.insert(m);
                                }
                                std::collections::btree_map::Entry::Occupied(mut m) => {
                                    m.get_mut().insert(pflow_fields_hash);
                                }
                            }
                        }
                    }
                    _ => {
                        self.graph.add_node(*ovhash);
                        self.vertex.insert(*ovhash, overtex.clone());
                        let mut m = BTreeSet::new();
                        m.insert(*ovhash);
                        mapping.insert(*ovhash, m);
                    }
                }
            } else {
                let mut m = BTreeSet::new();
                m.insert(*ovhash);
                mapping.insert(*ovhash, m);
            }
        }

        for (ofromhash, otohash, edgew) in other.graph.all_edges() {
            for ofromhash in &mapping[&ofromhash] {
                for otohash in &mapping[&otohash] {
                    for edge in edgew {
                        self.add_edge(*ofromhash, *otohash, *edge);
                        if matches!(edge.flow_type(), FlowType::Explicit) {
                            // TODO: link to the context through implicit links
                        }
                    }
                }
            }
        }

        Ok(mapping)
    }

    pub(crate) fn to_dot(&self, repo: &Repo) -> FlowResult<String> {
        let mut res = String::new();
        res.push_str("digraph {\n");
        res.push_str("  splines=ortho;\n");
        res.push_str("  nodesep=2;\n");
        write!(
            res,
            "{}",
            petgraph::dot::Dot::with_attr_getters(
                &self.graph,
                &[petgraph::dot::Config::GraphContentOnly],
                &|_, (_, _, edgew)| { format!("label=\"{}\"", edgew.dot_label(repo)) },
                &|_, (node, _)| {
                    let vertex = self.vertex(node);
                    format!("shape=box, label=\"{}\"", vertex.dot_label(repo))
                },
            )
        )
        .unwrap();
        res.push('}');
        Ok(res)
    }

    pub(crate) fn prune(&mut self, to_keep: Flows) {
        let to_keep = to_keep.nodes();
        let mut to_remove = BTreeSet::new();
        for hash in self.vertex.keys() {
            if self
                .graph
                .neighbors_directed(*hash, petgraph::Incoming)
                .count()
                == 0
                && self
                    .graph
                    .neighbors_directed(*hash, petgraph::Outgoing)
                    .count()
                    == 0
                && !to_keep.contains(hash)
            {
                to_remove.insert(*hash);
            }
        }
        for hash in to_remove.into_iter() {
            self.graph.remove_node(hash);
            self.vertex.remove(&hash);
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct VertexHash(u128);

impl fmt::Display for VertexHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash)]
pub(crate) enum Vertex {
    Null,
    Constant(MethodUid, Addr /* pc */),
    Parameter(MethodUid, usize /* param */, FieldSuffix),
    Instance(MethodUid, Addr /* pc */, FieldSuffix),
    Static(ClassUid, FieldSuffix),
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Constant(meth, pc) => write!(f, "{meth}.const_{pc}"),
            Self::Parameter(meth, param, fields) => write!(f, "{meth}.param_{param}{fields}"),
            Self::Instance(meth, pc, fields) => write!(f, "{meth}.obj_{pc}{fields}"),
            Self::Static(class, fields) => write!(f, "static {class}{fields}"),
        }
    }
}

impl Vertex {
    pub(crate) fn null() -> Self {
        Self::Null
    }

    pub(crate) fn constant(method: MethodUid, pc: Addr) -> Self {
        Self::Constant(method, pc)
    }

    pub(crate) fn parameter(method: MethodUid, param: usize) -> Self {
        Self::Parameter(method, param, FieldSuffix::default())
    }

    pub(crate) fn instance(method: MethodUid, pc: Addr) -> Self {
        Self::Instance(method, pc, FieldSuffix::default())
    }

    pub(crate) fn static_(class: ClassUid) -> Self {
        Self::Static(class, FieldSuffix::default())
    }

    pub(crate) fn field(&self, field: FieldUid) -> FlowResult<Self> {
        match self {
            Self::Null | Self::Constant(_, _) => Err(FlowError::InvalidFieldAccess),
            Self::Parameter(m, p, f) => Ok(Self::Parameter(*m, *p, f.clone().push(field))),
            Self::Instance(m, p, f) => Ok(Self::Instance(*m, *p, f.clone().push(field))),
            Self::Static(c, f) => Ok(Self::Static(*c, f.clone().push(field))),
        }
    }

    pub(crate) fn fields(&self, fields: &FieldSuffix) -> FlowResult<Self> {
        match self {
            Self::Null | Self::Constant(_, _) => {
                if fields.is_empty() {
                    Ok(self.clone())
                } else {
                    Err(FlowError::InvalidFieldAccess)
                }
            }
            Self::Parameter(m, p, f) => {
                Ok(Self::Parameter(*m, *p, f.clone().append(fields.clone())))
            }
            Self::Instance(m, p, f) => Ok(Self::Instance(*m, *p, f.clone().append(fields.clone()))),
            Self::Static(c, f) => Ok(Self::Static(*c, f.clone().append(fields.clone()))),
        }
    }

    pub(crate) fn dot_label(&self, repo: &Repo) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Constant(meth, pc) => format!("{}.const_{pc}", repo[*meth].descriptor()),
            Self::Parameter(meth, param, fields) => format!(
                "{}.param_{param}{}",
                repo[*meth].descriptor(),
                fields.dot_label(repo)
            ),
            Self::Instance(meth, pc, fields) => {
                format!(
                    "{}.obj_{pc}{}",
                    repo[*meth].descriptor(),
                    fields.dot_label(repo)
                )
            }
            Self::Static(class, fields) => format!("{class}.static{}", fields.dot_label(repo)),
        }
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub(crate) struct FieldSuffix(Vec<FieldUid>);

impl fmt::Display for FieldSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for field in &self.0 {
            write!(f, ".{field}")?;
        }
        Ok(())
    }
}

impl FieldSuffix {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn push(mut self, field: FieldUid) -> Self {
        self.0.push(field);
        self
    }

    fn append(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }

    fn dot_label(&self, repo: &Repo) -> String {
        let mut res = String::new();
        for field in &self.0 {
            res = format!("{res}.{}", repo[*field].name());
        }
        res
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EdgeWeight(BTreeSet<Edge>);

impl fmt::Display for EdgeWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for edge in &self.0 {
            write!(f, " {}", edge)?;
        }
        write!(f, " }}")
    }
}

impl From<Edge> for EdgeWeight {
    fn from(edge: Edge) -> Self {
        let mut res = BTreeSet::new();
        res.insert(edge);
        Self(res)
    }
}

impl<'a> IntoIterator for &'a EdgeWeight {
    type Item = <&'a BTreeSet<Edge> as IntoIterator>::Item;
    type IntoIter = <&'a BTreeSet<Edge> as IntoIterator>::IntoIter;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.iter()
    }
}

impl EdgeWeight {
    fn insert(&mut self, edge: Edge) {
        self.0.insert(edge);
    }

    fn join(&mut self, other: &Self) {
        for oedge in other {
            self.0.insert(*oedge);
        }
    }

    fn dot_label(&self, repo: &Repo) -> String {
        let mut res = String::new();
        for edge in &self.0 {
            res = format!("{res} {}", edge.dot_label(repo));
        }
        res
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Edge(FieldUid, FlowType);

impl fmt::Display for Edge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl Edge {
    pub(crate) fn field(field: FieldUid, flow: FlowType) -> Self {
        Self(field, flow)
    }

    pub(crate) fn field_uid(&self) -> FieldUid {
        self.0
    }

    pub(crate) fn flow_type(&self) -> FlowType {
        self.1
    }

    fn dot_label(&self, repo: &Repo) -> String {
        format!("({}, {})", repo[self.0].name(), self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FlowType {
    Explicit,
    Implicit,
}

impl fmt::Display for FlowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Explicit => write!(f, "Explicit"),
            Self::Implicit => write!(f, "Implicit"),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Flows(BTreeSet<Flow>);

impl fmt::Display for Flows {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        for flow in &self.0 {
            write!(f, " {flow}")?;
        }
        write!(f, " }}")
    }
}

impl Flows {
    pub(crate) fn add(&mut self, f: Flow) {
        let _ = self.0.insert(f);
    }

    fn subseteq(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    pub(crate) fn join(&self, other: &Self) -> Self {
        if self.subseteq(other) {
            return other.clone();
        }
        if other.subseteq(self) {
            return self.clone();
        }
        let mut res = self.0.clone();
        res.append(&mut other.0.clone());
        Self(res)
    }

    pub(crate) fn apply_context(&mut self, other: &BTreeSet<VertexHash>) {
        for vhash in other {
            self.add(Flow::new(*vhash, FlowType::Implicit))
        }
    }

    pub(crate) fn split(self) -> (Self, Self) {
        let mut res_e = BTreeSet::new();
        let mut res_i = BTreeSet::new();
        for f in self.0.into_iter() {
            if f.is_explicit() {
                res_e.insert(f);
            } else {
                res_i.insert(f);
            }
        }
        (Self(res_e), Self(res_i))
    }

    pub(crate) fn into_implicit(self) -> Self {
        let mut res = BTreeSet::new();
        for f in self.0.into_iter() {
            res.insert(f.into_implicit());
        }
        Self(res)
    }

    pub(crate) fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Flow) -> bool,
    {
        self.0.retain(f)
    }

    pub(crate) fn nodes(&self) -> BTreeSet<VertexHash> {
        let mut res = BTreeSet::new();
        for flow in &self.0 {
            res.insert(flow.0);
        }
        res
    }
}

impl<'a> IntoIterator for &'a Flows {
    type Item = <&'a BTreeSet<Flow> as IntoIterator>::Item;
    type IntoIter = <&'a BTreeSet<Flow> as IntoIterator>::IntoIter;
    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for Flows {
    type Item = <BTreeSet<Flow> as IntoIterator>::Item;
    type IntoIter = <BTreeSet<Flow> as IntoIterator>::IntoIter;
    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        self.0.into_iter()
    }
}

impl From<Flow> for Flows {
    fn from(f: Flow) -> Self {
        let mut res = BTreeSet::new();
        res.insert(f);
        Self(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Flow(VertexHash, FlowType);

impl fmt::Display for Flow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl Flow {
    pub(crate) fn new(vhash: VertexHash, ftype: FlowType) -> Self {
        Self(vhash, ftype)
    }

    fn into_implicit(self) -> Self {
        Self(self.0, FlowType::Implicit)
    }

    pub(crate) fn vertex_hash(&self) -> VertexHash {
        self.0
    }

    pub(crate) fn flow_type(&self) -> FlowType {
        self.1
    }

    fn is_implicit(&self) -> bool {
        matches!(self.1, FlowType::Implicit)
    }

    fn is_explicit(&self) -> bool {
        matches!(self.1, FlowType::Explicit)
    }
}
