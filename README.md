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

## PDF Rendering

NexusView renders PDF content by utilizing the pdfium binaries. To enable PDF preview, you need to download the pdfium-binaries from `https://github.com/bblanchon/pdfium-binaries/releases` and grab appropriate binary for your system (for linux I have used `pdfium-v8-linux-x64.tgz`) and then copy the extracted binaries into from `pdfium/lib/libpdfium.so` to `/target/debug/` or for system-wide `/usr/lib/` directory.

---

## TODO

### Core Graph Functionality

- [x] Directed Edges (Arrows)
- [x] Fix graph zooming (But only with buttons, no mouse wheel)
- [x] Fix graph panning (use middle mouse button)
- [ ] Fix Focus on Node (re-center and zoom on selected node)
- [ ] Graph mode link and tag in one with toggle
- [ ] "Show Orphans" unexpected behavior
- [ ] Graph Search `Next` and `Previous` buttons are doing unexpected behavior
- [ ] Context menu unexpected behavior

### Graph Interaction & Visualization

- [x] Dynamic Physics Parameters (UI controls for damping, spring constant, etc.)
- [x] Layout Reset (button to reset node positions)
- [x] Graph Search (find nodes by filename or content)
- [ ] Colors based on file type/tags
- [ ] Node Grouping/Clustering
- [ ] Weighted Edges
- [ ] Layout Algorithm Selection (e.g., Fruchterman-Reingold, Kamada-Kawai, grid, circular)
- [ ] "Freeze" Physics (button to pause simulation)
- [x] Toggle hidden files (`.file` and `.dir`)
- [ ] Fix toggle hidden files whole `pwd` is being rescanned not `selected directory`
- [ ] Fix text and node luminance
- [x] Fix when node is being dragged make other nodes slow
- [ ] Fix when node is being dragged make other nodes dim and change color.

### Data & Content

- [x] Syntax Highlighting (for code files in preview)
- [x] Markdown Rendering (render Markdown content in preview)
- [ ] Relative Path Links (full support for `../` and `./` links)
- [ ] URL/External Links (identify and open external URLs)
- [x] PDF/Document Preview (snippets or basic rendering) : using pdfuim (`https://github.com/bblanchon/pdfium-binaries/releases`)
- [ ] Add metadata display on top of pdf
- [ ] Improve PDF rendering
- [x] Add PDF scrolling instead of buttons
- [ ] Tag Cloud/List with frequency

### Configuration & Persistence

- [ ] Save/Load Layout (persist user-arranged graph layouts)
- [ ] Persistent Settings (save app settings between sessions)
- [ ] Config File (for custom settings)

### Usability & Polish

- [x] Progress Indicators (for scanning and graph building)
- [x] Context Menus (right-click actions on nodes/files)
- [ ] Improved Error Handling & Messaging
- [ ] Drag and Drop (files into app, or nodes for re-linking)
- [ ] Multi-selection of Nodes

### Performance & Scalability

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
