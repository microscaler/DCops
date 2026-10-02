# DCops UI

The single DCops web application: user and contributor documentation, the platform
dashboard (auto-discovered `dcops.microscaler.io` CRDs), and the operational sections
(NetBox inventory, IPAM, Boot, Certificates). Built with SolidJS, Vite and Tailwind CSS.

> **Consolidation (2026-09):** the former `ui-docs/` and `ui-dashboard/` apps were
> byte-for-byte subsets of this folder (same components, same content, drifted copies of
> `k8s-client.ts`). They were removed, along with `dockerfiles/Dockerfile.dcops-{docs,dashboard}`
> and `config/dcops-{docs,dashboard}`. This folder, `dockerfiles/Dockerfile.dcops-ui` and
> `config/dcops-ui` are the only UI build and deployment. Tilt runs it as `build-ui` on port 8800.
> Screen designs for every section live in `docs/03_ui_design/`.

## Development

```bash
# Install dependencies
yarn install

# Start development server
yarn dev

# Build for production
yarn build

# Preview production build
yarn preview
```

## Project Structure

```
ui/
├── src/
│   ├── components/
│   │   ├── layout/        # Navigation, ContentArea, Breadcrumbs, TableOfContents
│   │   └── content/       # MarkdownRenderer
│   ├── data/
│   │   ├── sections.ts    # Documentation structure
│   │   └── content/       # Markdown content files
│   ├── App.tsx
│   └── index.tsx
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

## Documentation Organization

Documentation is organized into two main categories:

- **User Documentation** - For end users
- **Contributor Documentation** - For developers

Content is stored in `src/data/content/` organized by category and section.

## Tech Stack

- **SolidJS** - Reactive UI framework
- **Vite** - Build tool
- **Tailwind CSS** - Styling
- **Marked** - Markdown parsing
- **Mermaid** - Diagram rendering

