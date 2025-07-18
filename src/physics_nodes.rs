// src/physics_nodes.rs
use egui::Vec2;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use rayon::prelude::*;
use std::collections::HashMap;

use crate::graph::GraphNode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsNode {
    pub pos: egui::Vec2,
    pub vel: egui::Vec2,
}

impl PhysicsNode {
    pub fn new(pos: egui::Vec2) -> Self {
        Self {
            pos,
            vel: egui::Vec2::ZERO,
        }
    }
}

pub struct PhysicsSimulator {
    pub node_positions: HashMap<NodeIndex, egui::Vec2>,
    pub node_velocities: HashMap<NodeIndex, egui::Vec2>,
    pub damping: f32,
    pub spring_constant: f32,
    pub repulsion_constant: f32,
    pub ideal_edge_length: f32,
    pub time_step: f32,
    pub friction: f32,
    pub frozen: bool,
}

impl PhysicsSimulator {
    pub fn new() -> Self {
        Self {
            node_positions: HashMap::new(),
            node_velocities: HashMap::new(),
            damping: 0.55,
            spring_constant: 0.3,
            repulsion_constant: 18000.0,
            ideal_edge_length: 180.0,
            time_step: 0.3,
            friction: 0.4,
            frozen: false,
        }
    }

    pub fn initialize_velocities(&mut self) {
        for node in self.node_positions.keys() {
            self.node_velocities.insert(*node, Vec2::ZERO);
        }
    }

    pub fn update(&mut self, edges: &[(NodeIndex, NodeIndex)]) {
        if self.frozen {
            return;
        }

        let node_indices: Vec<_> = self.node_positions.keys().copied().collect();
        let mut forces: HashMap<NodeIndex, Vec2> = HashMap::new();

        // Initialize forces to zero
        for &node in &node_indices {
            forces.insert(node, Vec2::ZERO);
        }

        // Parallel force calculation
        let (spring_forces, repulsion_forces) = rayon::join(
            || {
                let mut spring_forces = HashMap::new();
                for &(node1, node2) in edges {
                    if let (Some(&pos1), Some(&pos2)) = (
                        self.node_positions.get(&node1),
                        self.node_positions.get(&node2),
                    ) {
                        let delta = Vec2::new(pos2.x - pos1.x, pos2.y - pos1.y);
                        let distance = delta.length().max(0.1);
                        let displacement = distance - self.ideal_edge_length;

                        let force_magnitude = self.spring_constant * displacement;
                        let spring_force = (delta / distance) * force_magnitude;

                        *spring_forces.entry(node1).or_insert(Vec2::ZERO) += spring_force;
                        *spring_forces.entry(node2).or_insert(Vec2::ZERO) -= spring_force;
                    }
                }
                spring_forces
            },
            || {
                let mut repulsion_forces = HashMap::new();
                for i in 0..node_indices.len() {
                    for j in (i + 1)..node_indices.len() {
                        let node1 = node_indices[i];
                        let node2 = node_indices[j];

                        if let (Some(&pos1), Some(&pos2)) = (
                            self.node_positions.get(&node1),
                            self.node_positions.get(&node2),
                        ) {
                            let delta = Vec2::new(pos2.x - pos1.x, pos2.y - pos1.y);
                            let distance_sq = delta.length_sq();
                            let distance = distance_sq.sqrt().max(0.1);

                            let repulsion_force = (delta / distance)
                                * (self.repulsion_constant / distance_sq.max(10.0));

                            *repulsion_forces.entry(node1).or_insert(Vec2::ZERO) -= repulsion_force;
                            *repulsion_forces.entry(node2).or_insert(Vec2::ZERO) += repulsion_force;
                        }
                    }
                }
                repulsion_forces
            },
        );

        // Combine all forces
        for (node, force) in spring_forces {
            *forces.entry(node).or_default() += force;
        }
        for (node, force) in repulsion_forces {
            *forces.entry(node).or_default() += force;
        }

        // Update velocities and positions
        for (node_idx, force) in forces {
            if let (Some(pos), Some(vel)) = (
                self.node_positions.get_mut(&node_idx),
                self.node_velocities.get_mut(&node_idx),
            ) {
                *vel += force * self.time_step;
                *vel *= self.damping;
                *vel *= 1.0 - self.friction;
                *pos += *vel * self.time_step;
            }
        }
    }

    pub fn get_node_position(&self, index: NodeIndex) -> Option<&egui::Vec2> {
        self.node_positions.get(&index)
    }

    pub fn set_node_position(&mut self, index: NodeIndex, new_pos: egui::Vec2) {
        if let Some(pos) = self.node_positions.get_mut(&index) {
            *pos = new_pos;
            self.node_velocities.insert(index, egui::Vec2::ZERO);
        }
    }

    pub fn set_node_velocity(&mut self, index: NodeIndex, new_vel: egui::Vec2) {
        self.node_velocities.insert(index, new_vel);
    }

    pub fn reset_positions(&mut self, initial_layout: &HashMap<NodeIndex, egui::Vec2>) {
        self.node_positions = initial_layout.clone();
        self.initialize_velocities();
    }

    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
    }

    pub fn set_spring_constant(&mut self, spring_constant: f32) {
        self.spring_constant = spring_constant.max(0.0);
    }

    pub fn set_repulsion_constant(&mut self, repulsion_constant: f32) {
        self.repulsion_constant = repulsion_constant.max(0.0);
    }

    pub fn set_ideal_edge_length(&mut self, length: f32) {
        self.ideal_edge_length = length.max(0.0);
    }

    pub fn set_time_step(&mut self, time_step: f32) {
        self.time_step = time_step.max(0.0);
    }

    pub fn update_positions(&mut self) {}

    pub fn apply_forces(&mut self, nodes: &[NodeIndex], graph: &StableGraph<GraphNode, ()>) {}

    pub fn initialize_positions_from_graph(
        &mut self,
        graph: &StableGraph<GraphNode, ()>,
        center: egui::Vec2,
    ) {
    }
}
