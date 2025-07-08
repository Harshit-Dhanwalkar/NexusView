# NexusView

Visualize and explore connections between your files in an interactive knowledge graph. Written in Rust.

## About

NexusView is a desktop application built with Rust and the `egui` framework, designed to help you understand the relationships between your local files. It scans a specified directory, identifies links between files (e.g., Markdown links), and extracts tags, then presents this information as an interactive graph.

## Features

- **Interactive Graph Visualization**
- **Dual View Modes** (Link Graph and Tag Graph)
- **File Content Preview** (text and images)
- **Smart Filtering** (paths, tags, orphans)
- **Physics Simulation Controls**
- **Built with Rust** for performance

## Getting Started

### Prerequisites

- Rust 1.70+ ([rustup](https://rustup.rs/))

### Installation

```bash
git clone https://github.com/Harshit-Dhanwalkar/NexusView.git
cd NexusView
cargo run --release -- /path/to/your/directory
```

---

## Todo

- [ ] Fix graph zooming
- [ ] Fix graph panning
- [ ] "Show Orphans" unexpected behavior
- [ ] "Show Images" toggle responsiveness
- [ ] Fix Focus on Node (re-center and zoom on selected node)

- [ ] **Graph Interaction and Visualization Enhancements:**
  - [ ] Colors based on file type/tags
  - [ ] Node Grouping/Clustering
  - [ ] Weighted Edges
  - [x] Directed Edges (Arrows)
  - [ ] Layout Algorithm Selection (e.g., Fruchterman-Reingold, Kamada-Kawai, grid, circular)
  - [x] Dynamic Physics Parameters (UI controls for damping, spring constant, etc.)
  - [ ] "Freeze" Physics (button to pause simulation)
  - [x] Layout Reset (button to reset node positions)
  - [ ] Graph Search (find nodes by filename or content)
- [ ] **Data and Content Enhancements:**
  - [ ] Relative Path Links (full support for `../` and `./` links)
  - [ ] URL/External Links (identify and open external URLs)
  - [ ] Syntax Highlighting (for code files in preview)
  - [ ] Markdown Rendering (render Markdown content in preview)
  - [ ] PDF/Document Preview (snippets or basic rendering)
  - [ ] Tag Cloud/List with frequency
- [ ] **Configuration and Persistence:**
  - [ ] Save/Load Layout (persist user-arranged graph layouts)
  - [ ] Persistent Settings (save app settings between sessions)
  - [ ] Config File (for custom settings)
- [ ] **Usability and Polish:**
  - [ ] Progress Indicators (for scanning and graph building)
  - [ ] Improved Error Handling & Messaging
  - [ ] Context Menus (right-click actions on nodes/files)
  - [ ] Drag and Drop (files into app, or nodes for re-linking)
  - [ ] Multi-selection of Nodes
- [ ] **Performance and Scalability:**
  - [ ] Lazy Loading/Virtualization (for large lists/graphs)
  - [ ] Optimized Physics (further performance improvements)
  - [ ] Incremental Scanning (detect file changes for updates)

---

## Contributing

Contributions are welcome! If you find a bug, have a feature request, or want to contribute code, please feel free to open an issue or submit a pull request.

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**. See the `[LICENSE](LICENSE)` file for more details.

By using, distributing, or modifying this software, you agree to the terms of the GPLv3. This license ensures that all derivative works of NexusView will also be open source under the same license, promoting a vibrant and collaborative open-source ecosystem.
