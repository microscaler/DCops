import { Component } from 'solid-js';
import { DocCategory, userSections, contributorSections } from '../../data/sections';

interface BreadcrumbsProps {
  category: DocCategory;
  section: string | null;
  page: string | null;
  onNavigate: (category: DocCategory, section: string | null, page: string | null) => void;
}

const Breadcrumbs: Component<BreadcrumbsProps> = (props) => {
  const getSectionTitle = () => {
    if (!props.section) return null;
    const sections = props.category === 'user' ? userSections : contributorSections;
    return sections.find(s => s.id === props.section)?.title || null;
  };

  const getPageTitle = () => {
    if (!props.section || !props.page) return null;
    const sections = props.category === 'user' ? userSections : contributorSections;
    const section = sections.find(s => s.id === props.section);
    return section?.pages.find(p => p.id === props.page)?.title || null;
  };

  const isHome = () => props.page === 'index' && !props.section;

  return (
    <nav aria-label="Breadcrumb" class="flex items-center gap-2 text-sm">
      <button
        onClick={() => props.onNavigate(props.category, null, 'index')}
        class={`hover:text-[#4a5a4c] transition-colors ${
          isHome() ? 'text-[#4a5a4c] font-medium' : 'text-[#6b7280]'
        }`}
      >
        {props.category === 'user' ? 'User Docs' : 'Contributor Docs'}
      </button>
      <Show when={props.section}>
        <span class="text-[#9ca3af]">/</span>
        <button
          onClick={() => props.onNavigate(props.category, props.section, null)}
          class="text-[#6b7280] hover:text-[#4a5a4c] transition-colors"
        >
          {getSectionTitle()}
        </button>
      </Show>
      <Show when={props.page && props.page !== 'index'}>
        <span class="text-[#9ca3af]">/</span>
        <span class="text-[#4a5a4c] font-medium">
          {getPageTitle()}
        </span>
      </Show>
    </nav>
  );
};

export default Breadcrumbs;

