use egui::{Pos2, Vec2};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

pub struct PhysicsSimulator {
    pub node_positions: HashMap<NodeIndex, Pos2>,
    pub node_velocities: HashMap<NodeIndex, Vec2>,
    pub node_forces: HashMap<NodeIndex, Vec2>,
    pub damping: f32,            // Friction coefficient (0.0-1.0)
    pub spring_constant: f32,    // Controls edge stiffness
    pub repulsion_constant: f32, // Controls node repulsion
    pub ideal_edge_length: f32,  // Target length for edges
    pub time_step: f32,          // Simulation time step
}

impl PhysicsSimulator {
    pub fn new() -> Self {
        Self {
            node_positions: HashMap::new(),
            node_velocities: HashMap::new(),
            node_forces: HashMap::new(),
            damping: 0.3,
            spring_constant: 0.1,
            repulsion_constant: 1000.0,
            ideal_edge_length: 100.0,
            time_step: 0.5, // Slower simulation for stability
        }
    }

    pub fn initialize_velocities(&mut self) {
        for node in self.node_positions.keys() {
            self.node_velocities.insert(*node, Vec2::ZERO);
        }
    }

    pub fn update(&mut self, edges: &[(NodeIndex, NodeIndex)]) {
        self.clear_forces();
        self.calculate_spring_forces(edges);
        self.calculate_repulsion_forces();
        self.update_positions();
    }

    fn clear_forces(&mut self) {
        for node in self.node_positions.keys() {
            self.node_forces.insert(*node, Vec2::ZERO);
        }
    }

    fn calculate_spring_forces(&mut self, edges: &[(NodeIndex, NodeIndex)]) {
        for &(source, target) in edges {
            if let (Some(&start_pos), Some(&end_pos)) = (
                self.node_positions.get(&source),
                self.node_positions.get(&target),
            ) {
                let delta = end_pos.to_vec2() - start_pos.to_vec2();
                let distance = delta.length();
                let direction = delta.normalized();

                // Hooke's law: F = -kx
                let spring_force =
                    direction * (distance - self.ideal_edge_length) * self.spring_constant;

                *self.node_forces.entry(source).or_default() += spring_force;
                *self.node_forces.entry(target).or_default() -= spring_force;
            }
        }
    }

    fn calculate_repulsion_forces(&mut self) {
        let nodes: Vec<NodeIndex> = self.node_positions.keys().cloned().collect();

        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                let node1 = nodes[i];
                let node2 = nodes[j];

                if let (Some(&pos1), Some(&pos2)) = (
                    self.node_positions.get(&node1),
                    self.node_positions.get(&node2),
                ) {
                    let delta = pos2.to_vec2() - pos1.to_vec2();
                    let distance_sq = delta.length_sq();
                    let min_distance = 10.0; // Minimum distance to prevent extreme forces
                    let effective_distance_sq = distance_sq.max(min_distance * min_distance);

                    // Coulomb's law: F = k / r²
                    let repulsion_force = (delta / effective_distance_sq.sqrt())
                        * (self.repulsion_constant / effective_distance_sq);

                    *self.node_forces.entry(node1).or_default() -= repulsion_force;
                    *self.node_forces.entry(node2).or_default() += repulsion_force;
                }
            }
        }
    }

    fn update_positions(&mut self) {
        // Collect nodes first to avoid borrowing issues
        let nodes: Vec<NodeIndex> = self.node_positions.keys().cloned().collect();

        for node in nodes {
            if let (Some(force), Some(velocity), Some(position)) = (
                self.node_forces.get_mut(&node),
                self.node_velocities.get_mut(&node),
                self.node_positions.get_mut(&node),
            ) {
                // Apply force to velocity (F = ma, assuming m=1)
                *velocity += *force * self.time_step;

                // Apply damping (friction)
                *velocity *= 1.0 - (self.damping * self.time_step);

                // Update position
                *position = Pos2::new(
                    position.x + velocity.x * self.time_step,
                    position.y + velocity.y * self.time_step,
                );

                // Stop very small movements
                if velocity.length() < 0.1 {
                    *velocity = Vec2::ZERO;
                }
            }
        }
    }

    // Helper methods for parameter adjustment
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
        self.ideal_edge_length = length.max(1.0);
    }

    pub fn set_time_step(&mut self, time_step: f32) {
        self.time_step = time_step.max(0.01).min(1.0);
    }
}
