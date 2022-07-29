use std::marker::PhantomData;

use crate::db::cluster::{Cluster, ClusterNode};

pub struct DynCluster<N, C>
where
    N: 'static + Send + ClusterNode,
    C: Cluster<N>,
{
    cluster: C,
    node_type: PhantomData<N>,
}

pub struct DynNode {
    node: Box<dyn 'static + Send + ClusterNode>,
}

impl ClusterNode for DynNode {
    fn exec_session(&self, hist: &mut crate::db::history::Session) {
        self.node.exec_session(hist)
    }
}

impl<N, C> Cluster<DynNode> for DynCluster<N, C>
where
    N: 'static + Send + ClusterNode,
    C: Cluster<N>,
{
    fn n_node(&self) -> usize {
        self.cluster.n_node()
    }

    fn setup(&self) -> bool {
        self.cluster.setup()
    }

    fn setup_test(&mut self, p: &crate::db::history::HistParams) {
        self.cluster.setup_test(p)
    }

    fn get_node(&self, id: usize) -> crate::db::cluster::Node {
        self.cluster.get_node(id)
    }

    fn get_cluster_node(&self, id: usize) -> DynNode {
        DynNode {
            node: Box::new(self.cluster.get_cluster_node(id)),
        }
    }

    fn cleanup(&self) {
        self.cluster.cleanup()
    }

    fn info(&self) -> String {
        self.cluster.info()
    }
}

impl<N, C> DynCluster<N, C>
where
    N: 'static + Send + ClusterNode,
    C: Cluster<N>,
{
    pub fn new(cluster: C) -> Self {
        DynCluster {
            cluster,
            node_type: PhantomData::default(),
        }
    }
}
