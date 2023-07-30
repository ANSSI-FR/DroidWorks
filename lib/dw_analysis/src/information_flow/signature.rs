use crate::information_flow::amg;

use crate::information_flow::errors::FlowResult;
use crate::repo::Repo;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Signature {
    amg: amg::Amg,
    return_flows: amg::Flows,
    throw_flows: amg::Flows,
}

impl Signature {
    pub(crate) fn new(amg: amg::Amg) -> Self {
        Self {
            amg,
            return_flows: amg::Flows::default(),
            throw_flows: amg::Flows::default(),
        }
    }

    pub(crate) fn amg(&self) -> &amg::Amg {
        &self.amg
    }

    pub(crate) fn amg_mut(&mut self) -> &mut amg::Amg {
        &mut self.amg
    }

    pub(crate) fn join_return(&mut self, flows: &amg::Flows) {
        self.return_flows = self.return_flows.join(flows);
    }

    pub(crate) fn join(&mut self, other: &Self) {
        self.amg.join(&other.amg);
        self.return_flows = self.return_flows.join(&other.return_flows);
        self.throw_flows = self.return_flows.join(&other.throw_flows);
    }

    pub(crate) fn inject(
        &mut self,
        other: &Self,
        parameter_flows: Vec<amg::Flows>,
    ) -> FlowResult<InjectionOutput> {
        let mapping = self.amg.inject(&other.amg, parameter_flows)?;

        let mut return_flows = amg::Flows::default();
        for flow in &other.return_flows {
            let fhash = flow.vertex_hash();
            match mapping.get(&fhash) {
                None => return_flows.add(amg::Flow::new(fhash, flow.flow_type())),
                Some(shashes) => {
                    for shash in shashes {
                        return_flows.add(amg::Flow::new(*shash, flow.flow_type()));
                    }
                }
            }
        }

        let mut throw_flows = amg::Flows::default();
        for flow in &other.throw_flows {
            let fhash = flow.vertex_hash();
            match mapping.get(&fhash) {
                None => throw_flows.add(amg::Flow::new(fhash, flow.flow_type())),
                Some(shashes) => {
                    for shash in shashes {
                        throw_flows.add(amg::Flow::new(*shash, flow.flow_type()));
                    }
                }
            }
        }

        Ok(InjectionOutput {
            return_flows,
            throw_flows,
        })
    }

    pub fn pretty_print(&self, repo: &Repo) -> FlowResult<String> {
        let mut res = self.amg.to_dot(repo)?;
        res.push('\n');
        res.push_str(&format!("\nreturns = {}\n", self.return_flows));
        res.push_str(&format!("throws = {}", self.throw_flows));
        Ok(res)
    }

    pub fn prune(&mut self) {
        let to_keep = self.return_flows.join(&self.throw_flows);
        self.amg.prune(to_keep)
    }
}

pub(crate) struct InjectionOutput {
    pub(crate) return_flows: amg::Flows,
    pub(crate) throw_flows: amg::Flows,
}
