export type DocCategory = 'user' | 'contributor';

export interface DocSection {
  id: string;
  title: string;
  pages: DocPage[];
}

export interface DocPage {
  id: string;
  title: string;
  file: string;
}

export const userSections: DocSection[] = [
  {
    id: 'getting-started',
    title: 'Getting Started',
    pages: [
      { id: 'overview', title: 'Overview', file: 'getting-started/overview.md' },
      { id: 'installation', title: 'Installation', file: 'getting-started/installation.md' },
      { id: 'quick-start', title: 'Quick Start', file: 'getting-started/quick-start.md' },
    ],
  },
  {
    id: 'concepts',
    title: 'Core Concepts',
    pages: [
      { id: 'gitops-workflow', title: 'GitOps Workflow', file: 'concepts/gitops-workflow.md' },
      { id: 'ip-allocation', title: 'IP Address Allocation', file: 'concepts/ip-allocation.md' },
      { id: 'infrastructure-inventory', title: 'Infrastructure Inventory', file: 'concepts/infrastructure-inventory.md' },
      { id: 'pxe-boot', title: 'PXE Boot Control', file: 'concepts/pxe-boot.md' },
    ],
  },
  {
    id: 'guides',
    title: 'Guides',
    pages: [
      { id: 'netbox-setup', title: 'NetBox Setup', file: 'guides/netbox-setup.md' },
      { id: 'ip-pool-management', title: 'IP Pool Management', file: 'guides/ip-pool-management.md' },
      { id: 'site-management', title: 'Site Management', file: 'guides/site-management.md' },
      { id: 'pxe-configuration', title: 'PXE Configuration', file: 'guides/pxe-configuration.md' },
    ],
  },
  {
    id: 'api-reference',
    title: 'API Reference',
    pages: [
      { id: 'crd-reference', title: 'CRD Reference', file: 'api-reference/crd-reference.md' },
      { id: 'netbox-controller', title: 'NetBox Controller', file: 'api-reference/netbox-controller.md' },
      { id: 'pxe-intent-controller', title: 'PXE Intent Controller', file: 'api-reference/pxe-intent-controller.md' },
    ],
  },
];

export const contributorSections: DocSection[] = [
  {
    id: 'development',
    title: 'Development',
    pages: [
      { id: 'setup', title: 'Development Setup', file: 'development/setup.md' },
      { id: 'architecture', title: 'Architecture', file: 'development/architecture.md' },
      { id: 'testing', title: 'Testing', file: 'development/testing.md' },
    ],
  },
  {
    id: 'contributing',
    title: 'Contributing',
    pages: [
      { id: 'contributing-guide', title: 'Contributing Guide', file: 'contributing/contributing-guide.md' },
      { id: 'code-style', title: 'Code Style', file: 'contributing/code-style.md' },
    ],
  },
];

