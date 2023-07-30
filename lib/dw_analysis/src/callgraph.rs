//! Graph representations of the possible calls between classes methods
//! of an application and dependencies.

use crate::errors::AnalysisResult;
use crate::repo;
use dw_dex::instrs::Instr;
use dw_dex::registers::Reg;
use dw_dex::types::Type;
use dw_dex::{Addr, DexIndex};
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use petgraph::visit::{DfsPostOrder, NodeRef, Reversed};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryFrom;
use std::fmt::{self, Write};

#[derive(Debug, Clone)]
pub enum MethodDef<'a> {
    Method(&'a repo::Method<'a>),
    Descriptor(repo::MethodDescr),
}

impl<'a> fmt::Display for MethodDef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Method(m) => m.descriptor().fmt(f),
            Self::Descriptor(d) => d.fmt(f),
        }
    }
}

impl<'a> MethodDef<'a> {
    fn descriptor(&self) -> &repo::MethodDescr {
        match self {
            Self::Method(m) => m.descriptor(),
            Self::Descriptor(d) => d,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MethodStatus {
    App,
    System,
    Inherited,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Method<'a> {
    def: MethodDef<'a>,
    status: MethodStatus,
    zombie_roots: BTreeSet<Addr>,
    zombie_calls: BTreeSet<Addr>,
}

impl<'a> fmt::Display for Method<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.def.fmt(f)
    }
}

impl<'a> Method<'a> {
    fn new(def: MethodDef<'a>, status: MethodStatus) -> Self {
        Self {
            def,
            status,
            zombie_roots: BTreeSet::new(),
            zombie_calls: BTreeSet::new(),
        }
    }

    pub fn is_defined(&self) -> bool {
        matches!(self.def, MethodDef::Method(_))
    }

    pub fn is_zombie(&self) -> bool {
        !self.zombie_roots.is_empty() || !self.zombie_calls.is_empty()
    }

    pub fn class_name(&self) -> String {
        self.def.descriptor().definer().class_name()
    }

    pub fn name(&self) -> &str {
        self.def.descriptor().name()
    }
}

#[derive(Debug, Clone)]
pub struct Call {
    call_addrs: BTreeSet<Addr>,
}

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, addr) in self.call_addrs.iter().enumerate() {
            write!(f, "{addr}")?;
            if i < self.call_addrs.len() - 1 {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CallGraph<'a> {
    inner: DiGraph<Method<'a>, Call>,
}

impl<'a> CallGraph<'a> {
    /// Builds a callgraph for all the (non-system) methods contained in the given
    /// repository. The methods marked as system in the repository appear in the
    /// callgraph only if a non-system one calls (invokes) it.
    pub fn build(repo: &'a repo::Repo, unfold_system_methods: bool) -> AnalysisResult<Self> {
        let mut cg = DiGraph::new();
        let mut nodes_map: BTreeMap<repo::MethodDescr, NodeIndex> = BTreeMap::new();

        // create a graph node for each method of the repo
        for class in repo
            .iter_classes()
            .filter(|cl| cl.is_defined() && (unfold_system_methods || !cl.is_system()))
        {
            for method in class.iter_methods(repo) {
                nodes_map.insert(
                    method.descriptor().clone(),
                    cg.add_node(Method::new(MethodDef::Method(method), MethodStatus::App)),
                );
            }
        }

        // create edges for each call, and create graph nodes when non existing:
        //   - system method if unfold_system_methods not set and method not already in the graph but existing in the repo
        //   - zombie method if unfold_system_methods set or method not already in the graph and not found in the repo
        for class in repo
            .iter_classes()
            .filter(|cl| cl.is_defined() && (unfold_system_methods || !cl.is_system()))
        {
            for method in class
                .iter_methods(repo)
                .filter(|meth| meth.code().is_some())
            {
                let src = *nodes_map.get(method.descriptor()).unwrap(); // cannot panic due to first loop
                for (called, call_addrs) in compute_calls(method)? {
                    let dst = if let Some(id) = nodes_map.get(&called) {
                        *id
                    } else {
                        let m = if unfold_system_methods {
                            let m = Method::new(
                                MethodDef::Descriptor(called.clone()),
                                MethodStatus::Unknown,
                            );
                            log::trace!(
                                "marking {} as zombie since it tries to call method {} which cannot be found in repo",
                                method.descriptor(),
                                called,
                            );
                            for call_addr in &call_addrs {
                                cg[src].zombie_roots.insert(*call_addr);
                            }
                            m
                        } else if let Some(mdef) = repo.find_method_by_descriptor(&called) {
                            Method::new(MethodDef::Method(mdef), MethodStatus::System)
                        } else if repo.is_inherited(&called) {
                            Method::new(
                                MethodDef::Descriptor(called.clone()),
                                MethodStatus::Inherited,
                            )
                        } else {
                            let m = Method::new(
                                MethodDef::Descriptor(called.clone()),
                                MethodStatus::Unknown,
                            );
                            log::trace!(
                                "marking {} as zombie since it tries to call method {} which cannot be found in repo",
                                method.descriptor(),
                                called,
                            );
                            for call_addr in &call_addrs {
                                cg[src].zombie_roots.insert(*call_addr);
                            }
                            m
                        };
                        let id = cg.add_node(m);
                        nodes_map.insert(called, id);
                        id
                    };
                    cg.add_edge(src, dst, Call { call_addrs });
                }
            }
        }

        Ok(Self { inner: cg })
    }

    #[must_use]
    pub fn to_dot(&self) -> String {
        let mut res = String::new();
        res.push_str("digraph {\n");
        res.push_str("  rankdir=LR;\n");
        write!(
            res,
            "{}",
            Dot::with_attr_getters(
                &self.inner,
                &[Config::GraphContentOnly],
                &|_, _| String::new(),
                &|_, node| {
                    let m = node.weight();
                    let color = if matches!(m.status, MethodStatus::System) {
                        "blue"
                    } else if !m.is_defined() {
                        "red"
                    } else if m.is_zombie() {
                        "purple"
                    } else {
                        "black"
                    };
                    format!("color={color},shape=box")
                }
            )
        )
        .unwrap();
        res.push('}');
        res
    }

    pub fn mark_unknown_refs(&mut self, repo: &repo::Repo) -> AnalysisResult<()> {
        for id in self.inner.node_indices() {
            let mut new_zombie_roots = BTreeSet::new();
            if let MethodDef::Method(method) = &self.inner[id].def {
                if let Some(code) = method.code() {
                    for linstr in code.read().unwrap().iter_instructions() {
                        match linstr.instr() {
                            // TODO: for *newarray* instructions, destruct array type to get class
                            // name and check for its existence in repo
                            Instr::ConstClass(_, t)
                            | Instr::CheckCast(_, t)
                            | Instr::InstanceOf(_, _, t)
                            | Instr::NewInstance(_, t)
                            | Instr::NewArray(_, _, t)
                            | Instr::FilledNewArray(_, t)
                            | Instr::FilledNewArrayRange(_, t) => {
                                let dex = method.dex();
                                let typ = t.get(dex)?;
                                if let Type::Class(cl) = typ.to_type(dex)? {
                                    if repo.get_class_by_name(&cl).is_none() {
                                        new_zombie_roots.insert(linstr.addr());
                                    }
                                }
                            }
                            Instr::Iget(_, _, f)
                            | Instr::IgetWide(_, _, f)
                            | Instr::IgetObject(_, _, f)
                            | Instr::IgetBoolean(_, _, f)
                            | Instr::IgetByte(_, _, f)
                            | Instr::IgetChar(_, _, f)
                            | Instr::IgetShort(_, _, f)
                            | Instr::Iput(_, _, f)
                            | Instr::IputWide(_, _, f)
                            | Instr::IputObject(_, _, f)
                            | Instr::IputBoolean(_, _, f)
                            | Instr::IputByte(_, _, f)
                            | Instr::IputChar(_, _, f)
                            | Instr::IputShort(_, _, f)
                            | Instr::Sget(_, f)
                            | Instr::SgetWide(_, f)
                            | Instr::SgetObject(_, f)
                            | Instr::SgetBoolean(_, f)
                            | Instr::SgetByte(_, f)
                            | Instr::SgetChar(_, f)
                            | Instr::SgetShort(_, f)
                            | Instr::Sput(_, f)
                            | Instr::SputWide(_, f)
                            | Instr::SputObject(_, f)
                            | Instr::SputBoolean(_, f)
                            | Instr::SputByte(_, f)
                            | Instr::SputChar(_, f)
                            | Instr::SputShort(_, f) => {
                                let dex = method.dex();
                                let field = f.get(dex)?;
                                if let Some(class) = repo.get_class_by_name(&field.class_name(dex)?)
                                {
                                    if class
                                        .get_field(&field.name(dex)?, &field.type_(dex)?, repo)
                                        .is_none()
                                    {
                                        log::trace!(
                                            "marking {} as zombie since it tries to access field {}->{} of type {} which cannot be found in repo",
                                            method.descriptor(),
                                            field.class_name(dex)?,
                                            field.name(dex)?,
                                            field.type_(dex)?,
                                        );
                                        new_zombie_roots.insert(linstr.addr());
                                    }
                                } else {
                                    log::trace!(
                                        "marking {} as zombie since it tries to access field {}->{} of type {} which cannot be found in repo",
                                        method.descriptor(),
                                        field.class_name(dex)?,
                                        field.name(dex)?,
                                        field.type_(dex)?,
                                    );
                                    new_zombie_roots.insert(linstr.addr());
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            self.inner[id].zombie_roots.append(&mut new_zombie_roots);
        }
        Ok(())
    }

    pub fn propagate_zombies(&mut self) {
        let mut todo = BTreeSet::new();
        for id in self.inner.node_indices() {
            if self.inner[id].is_zombie() {
                todo.insert(id);
            }
        }

        let mut visited = BTreeSet::new();
        while !todo.is_empty() {
            let dst = todo.pop_first().unwrap();
            visited.insert(dst);
            let callers: Vec<(NodeIndex, BTreeSet<Addr>)> = self
                .inner
                .edges_directed(dst, Direction::Incoming)
                .map(|edge| (edge.source(), edge.weight().call_addrs.clone()))
                .collect();
            for (src, addrs) in callers {
                for addr in addrs {
                    self.inner[src].zombie_calls.insert(addr);
                }
                if !visited.contains(&src) {
                    todo.insert(src);
                }
            }
        }
    }

    pub fn patch_unknown_refs(&self, repo: &repo::Repo) -> AnalysisResult<()> {
        for node in self.inner.node_weights() {
            for addr in &node.zombie_roots {
                let method_descr = node.def.descriptor();
                let method = repo.find_method_by_descriptor(method_descr).unwrap();
                let code = method.code().unwrap();
                let (current_instr, next_instr) = {
                    let borrow = code.read().unwrap();
                    let current_instr = borrow.instruction_at(*addr)?.clone();

                    let next_instr = if let Ok(i) = borrow.instruction_at(current_instr.next_addr())
                    {
                        Some(i.clone())
                    } else {
                        None
                    };
                    (current_instr, next_instr)
                };
                if matches!(
                    current_instr.instr(),
                    Instr::FilledNewArray(_, _)
                        | Instr::FilledNewArrayRange(_, _)
                        | Instr::InvokeVirtual(_, _)
                        | Instr::InvokeSuper(_, _)
                        | Instr::InvokeDirect(_, _)
                        | Instr::InvokeStatic(_, _)
                        | Instr::InvokeInterface(_, _)
                        | Instr::InvokeVirtualRange(_, _)
                        | Instr::InvokeSuperRange(_, _)
                        | Instr::InvokeDirectRange(_, _)
                        | Instr::InvokeStaticRange(_, _)
                        | Instr::InvokeInterfaceRange(_, _)
                ) {
                    if let Some(ninstr) = next_instr {
                        if let Instr::MoveResultObject(dst) = ninstr.instr() {
                            code.write()
                                .unwrap()
                                .patch_instruction_at(*addr, vec![Instr::Const16(*dst, 0)])?;
                            code.write()
                                .unwrap()
                                .patch_instruction_at(current_instr.next_addr(), Vec::new())?;
                            continue;
                        }
                    }
                    code.write()
                        .unwrap()
                        .patch_instruction_at(*addr, Vec::new())?;
                    continue;
                }
                let new_instrs = match current_instr.instr() {
                    Instr::ConstClass(dst, _) => vec![Instr::Const16(*dst, 0)],
                    Instr::CheckCast(_, _) => vec![
                        Instr::Const4(Reg::from(0u8), 0),
                        Instr::Throw(Reg::from(0u8)),
                    ],
                    Instr::InstanceOf(dst, _, _) => vec![Instr::Const4(*dst, 0)],
                    Instr::NewInstance(dst, _) => vec![Instr::Const16(*dst, 0)],
                    Instr::NewArray(dst, _, _) => vec![Instr::Const4(*dst, 0)],
                    Instr::Iget(dst, _, _)
                    | Instr::IgetObject(dst, _, _)
                    | Instr::IgetBoolean(dst, _, _)
                    | Instr::IgetByte(dst, _, _)
                    | Instr::IgetChar(dst, _, _)
                    | Instr::IgetShort(dst, _, _)
                    | Instr::Sget(dst, _)
                    | Instr::SgetObject(dst, _)
                    | Instr::SgetBoolean(dst, _)
                    | Instr::SgetByte(dst, _)
                    | Instr::SgetChar(dst, _)
                    | Instr::SgetShort(dst, _) => vec![Instr::Const4(*dst, 0)],

                    Instr::IgetWide(dst, _, _) | Instr::SgetWide(dst, _) => {
                        vec![Instr::ConstWide16(*dst, 0)]
                    }

                    Instr::Iput(_, _, _)
                    | Instr::IputObject(_, _, _)
                    | Instr::IputBoolean(_, _, _)
                    | Instr::IputByte(_, _, _)
                    | Instr::IputChar(_, _, _)
                    | Instr::IputShort(_, _, _)
                    | Instr::Sput(_, _)
                    | Instr::SputObject(_, _)
                    | Instr::SputBoolean(_, _)
                    | Instr::SputByte(_, _)
                    | Instr::SputChar(_, _)
                    | Instr::SputShort(_, _)
                    | Instr::IputWide(_, _, _)
                    | Instr::SputWide(_, _) => Vec::new(),

                    instr => panic!("trying to patch instruction: {:?}", instr),
                };
                code.write()
                    .unwrap()
                    .patch_instruction_at(*addr, new_instrs)?;
            }
        }
        Ok(())
    }

    pub fn traverse_from_callees_to_callers(&self) -> CGRevIterator {
        CGRevIterator::new(&self.inner)
    }

    pub fn filter<P>(&self, predicate: P) -> Self
    where
        P: Fn(&Method) -> bool,
    {
        // Since we remove nodes while keeping ids collection,
        // we need to switch to the stable graph representation so
        // that ids are preserved.
        let mut stable_graph: StableDiGraph<_, _> = self.inner.clone().into();

        // collect all callgraph nodes
        let mut to_remove: BTreeSet<NodeIndex> = stable_graph.node_indices().collect();

        // remove from the collection all nodes from paths,
        // by doing a backward traversale from targets.
        let reversed = Reversed(&stable_graph);
        let mut dfs = Dfs::empty(reversed);
        for id in stable_graph.node_indices() {
            if predicate(&stable_graph[id]) {
                dfs.move_to(id);
                while let Some(keep_id) = dfs.next(reversed) {
                    to_remove.remove(&keep_id);
                }
            }
        }

        // remove from callgraph all remainings nodes.
        stable_graph.retain_nodes(|_, id| !to_remove.contains(&id));

        Self {
            inner: stable_graph.into(),
        }
    }

    pub fn nb_methods(&self) -> usize {
        self.inner.node_count()
    }

    pub fn nb_system_methods(&self) -> usize {
        self.inner
            .node_weights()
            .filter(|m| matches!(m.status, MethodStatus::System))
            .count()
    }

    pub fn nb_zombie_methods(&self) -> usize {
        self.inner.node_weights().filter(|m| m.is_zombie()).count()
    }
}

fn compute_calls(
    method: &repo::Method,
) -> AnalysisResult<BTreeMap<repo::MethodDescr, BTreeSet<Addr>>> {
    let mut map = BTreeMap::new();
    let Some(code) = method.code() else {
        return Ok(map);
    };
    let dex = method.dex();

    for instr in code.read().unwrap().iter_instructions() {
        match instr.instr() {
            Instr::InvokeVirtual(_, m)
            | Instr::InvokeSuper(_, m)
            | Instr::InvokeDirect(_, m)
            | Instr::InvokeStatic(_, m)
            | Instr::InvokeInterface(_, m)
            | Instr::InvokeVirtualRange(_, m)
            | Instr::InvokeSuperRange(_, m)
            | Instr::InvokeDirectRange(_, m)
            | Instr::InvokeStaticRange(_, m)
            | Instr::InvokeInterfaceRange(_, m) => {
                let descriptor = m.get(dex)?;
                let prototype = repo::MethodDescr::try_from((dex, descriptor))?;
                match map.get_mut(&prototype) {
                    None => {
                        let addrs = BTreeSet::from([instr.addr()]);
                        map.insert(prototype, addrs);
                    }
                    Some(addrs) => {
                        addrs.insert(instr.addr());
                    }
                }
            }
            _ => (),
        };
    }

    Ok(map)
}

pub struct CGRevIterator<'a> {
    graph: &'a DiGraph<Method<'a>, Call>,
    dfspo: DfsPostOrder<NodeIndex, fixedbitset::FixedBitSet>,
    roots: Vec<NodeIndex>,
}

impl<'a> CGRevIterator<'a> {
    fn new(graph: &'a DiGraph<Method<'a>, Call>) -> Self {
        let mut roots: Vec<NodeIndex> = graph
            .node_indices()
            .filter(|i| graph.edges_directed(*i, Incoming).next().is_none())
            .collect();
        let root = if let Some(root) = roots.pop() {
            root
        } else {
            graph.node_indices().next().expect("non empty graph")
        };
        Self {
            graph,
            dfspo: DfsPostOrder::new(&graph, root),
            roots,
        }
    }
}

impl<'a> Iterator for CGRevIterator<'a> {
    type Item = &'a Method<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.dfspo.next(&self.graph) {
            return Some(&self.graph[n]);
        }
        if let Some(root) = self.roots.pop() {
            self.dfspo.move_to(root);
            self.next()
        } else {
            None
        }
    }
}
