# NexusView

Visualize and explore connections between your files in an interactive knowledge graph. Written in Rust.

## About

NexusView is a desktop application built with Rust and the `egui` framework, designed to help you understand the relationships between your local files. It scans a specified directory, identifies links between files (e.g., Markdown links), and extracts tags, then presents this information as an interactive graph. Whether you're managing a personal knowledge base, research notes, or code documentation, NexusView provides a unique way to navigate and discover connections within your file system.

## Features

- **Interactive Graph Visualization:** Explore your file connections as a dynamic, and pannable graph.
- **Link Graph Mode:** See explicit links between your files (e.g., `[[internal links]]` in Markdown).
- **Tag Graph Mode:** Discover connections based on shared tags found within your files.
- **File List and Filtering:** Easily browse all scanned files and filter them by tags.
- **File Content Preview:** View the content of selected text-based files directly within the application.
- **Image Preview:** Display embedded images.
- **Configurable Path Display:** Toggle between showing full absolute paths or just filenames in the graph.
- **Built with Rust:** High performance and reliability.
- **Physics-based Node Simulation:**
  - **Dynamic Layout:** Nodes in the graph now interact with each other using a force-directed algorithm, providing a more organized and spread-out visualization.
  - **Configurable Physics:** The simulation uses parameters such as:
    - **Damping:** Controls the overall friction in the system, preventing oscillations.
    - **Spring Constant:** Determines the strength of the attractive force between connected nodes.
    - **Repulsion Constant:** Defines the repulsive force between all nodes, preventing overlap.
    - **Ideal Edge Length:** Sets the desired distance between connected nodes.
  - **Interactive Dragging:** You can drag individual nodes to reposition them, and the physics simulation will adapt accordingly, allowing for custom adjustments to the layout.

## Screenshots

![Screenshot of Tag Graph View](assets/tag-graph-veiw.png)
_A screenshot showing the Tag Graph view with interconnected nodes representing files._

![Screenshot of image preiew](assets/image-preview.png)
_A screenshot showing the Tag Graph view with interconnected nodes representing files._

## Getting Started

### Prerequisites

- Rust programming language (ensure you have `rustup` installed)

### Building and Running

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/Harshit-Dhanwalkar/NexusView.git
    cd NexusView
    ```
2.  **Build the application:**
    ```bash
    cargo build --release
    ```
3.  **Run the application:**

    ```bash
    # Replace `/path/to/your/notes` with the directory you want to scan
    ./target/release/interactive-fm /path/to/your/notes
    ```

    **Example:**

    ```bash
    ./target/release/interactive-fm ~/Documents/MyKnowledgeBase
    ```

## Usage

Upon launching NexusView with a specified directory, the application will scan its contents for files, links, and tags.

- **Left Panel:** Contains a list of all scanned files and images, along with a tag filter input. Clicking on a file here will show its content/image preview and select its corresponding node in the graph (if the graph view is open).
- **Top Panel:** Provides options to exit the application and toggle the graph visualization.
- **Central Panel (Graph View):**
  - **Pan:** Click and drag the background to move the graph.
  - **Nodes:** Represent your files. Click on a node to select it and view its details in the left panel.
  - **Edges:** Represent connections (links or shared tags) between files.
  - **Graph Type Radio Buttons:** Switch between "Links" (explicit file-to-file links) and "Tags" (files connected by shared tags).
  - **Path Display Toggle:** Switch between displaying full absolute paths or just filenames on the graph nodes.

## Todo

- [ ] Fix graph zooming
- [ ] **Graph Interaction and Visualization Enhancements:**
  - [ ] Colors based on file type/tags
  - [ ] Node Grouping/Clustering
  - [ ] Weighted Edges
  - [ ] Directed Edges (Arrows)
  - [ ] Layout Algorithm Selection (e.g., Fruchterman-Reingold, Kamada-Kawai, grid, circular)
  - [ ] Dynamic Physics Parameters (UI controls for damping, spring constant, etc.)
  - [ ] "Freeze" Physics (button to pause simulation)
  - [ ] Layout Reset (button to reset node positions)
  - [ ] Graph Search (find nodes by filename or content)
  - [ ] Focus on Node (re-center and zoom on selected node)
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

## Contributing

Contributions are welcome! If you find a bug, have a feature request, or want to contribute code, please feel free to open an issue or submit a pull request.

## License

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**. See the `[LICENSE](LICENSE)` file for more details.

By using, distributing, or modifying this software, you agree to the terms of the GPLv3. This license ensures that all derivative works of NexusView will also be open source under the same license, promoting a vibrant and collaborative open-source ecosystem.
