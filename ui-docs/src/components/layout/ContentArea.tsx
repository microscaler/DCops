import { Component, createSignal, createEffect } from 'solid-js';
import { DocCategory, userSections, contributorSections } from '../../data/sections';
import MarkdownRenderer from '../content/MarkdownRenderer';

interface ContentAreaProps {
  category: DocCategory;
  section: string | null;
  page: string | null;
  onContentChange?: (content: string) => void;
  onNavigate: (category: DocCategory, section: string | null, page: string | null) => void;
}

const ContentArea: Component<ContentAreaProps> = (props) => {
  const [content, setContent] = createSignal<string>('');
  const [loading, setLoading] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);

  const contentModules = import.meta.glob('../../data/content/**/*.md', { 
    eager: false,
    query: '?raw',
    import: 'default'
  });

  createEffect(() => {
    loadContent();
  });

  const loadContent = async () => {
    if (props.page === 'index' && !props.section) {
      setLoading(true);
      setError(null);
      try {
        const filePath = `../../data/content/${props.category}/index.md`;
        const module = contentModules[filePath];
        if (module) {
          const text = await module();
          setContent(text as string);
          props.onContentChange?.(text as string);
        } else {
          const placeholder = '# Welcome\n\nSelect a page from the navigation to get started.';
          setContent(placeholder);
          props.onContentChange?.(placeholder);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load content');
        const placeholder = '# Welcome\n\nSelect a page from the navigation to get started.';
        setContent(placeholder);
        props.onContentChange?.(placeholder);
      } finally {
        setLoading(false);
      }
      return;
    }

    if (!props.section || !props.page) {
      const placeholder = '# Welcome\n\nSelect a page from the navigation to get started.';
      setContent(placeholder);
      props.onContentChange?.(placeholder);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const sections = props.category === 'user' ? userSections : contributorSections;
      const section = sections.find(s => s.id === props.section);
      const pageDef = section?.pages.find(p => p.id === props.page);
      
      const filePath = pageDef 
        ? `../../data/content/${props.category}/${pageDef.file}`
        : `../../data/content/${props.category}/${props.section}/${props.page}.md`;
      
      const module = contentModules[filePath];
      if (module) {
        const text = await module();
        setContent(text as string);
        props.onContentChange?.(text as string);
      } else {
        setError(`Page not found: ${filePath}`);
        setContent(`# Page Not Found\n\nThe requested page could not be found.`);
        props.onContentChange?.('');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load content');
      setContent(`# Error\n\nFailed to load content: ${err instanceof Error ? err.message : 'Unknown error'}`);
      props.onContentChange?.('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <main class="flex-1 overflow-y-auto custom-scrollbar bg-white" role="main">
      <div class="max-w-4xl mx-auto px-8 py-10">
        <Show when={loading()}>
          <div class="text-center py-20">
            <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-[#4a5a4c]"></div>
            <p class="mt-4 text-[#6b7280]">Loading...</p>
          </div>
        </Show>
        <Show when={!loading() && error()}>
          <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
            <p class="text-red-800">{error()}</p>
          </div>
        </Show>
        <Show when={!loading()}>
          <div class="prose prose-slate max-w-none">
            <MarkdownRenderer content={content()} />
          </div>
        </Show>
      </div>
    </main>
  );
};

export default ContentArea;

