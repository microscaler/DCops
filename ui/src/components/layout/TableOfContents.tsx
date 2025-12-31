import { Component, createSignal, createEffect, For, Show } from 'solid-js';

interface TableOfContentsProps {
  content: string;
}

interface Heading {
  id: string;
  text: string;
  level: number;
}

const TableOfContents: Component<TableOfContentsProps> = (props) => {
  const [headings, setHeadings] = createSignal<Heading[]>([]);
  const [activeId, setActiveId] = createSignal<string>('');

  createEffect(() => {
    // Extract headings from content after a short delay to allow DOM updates
    setTimeout(() => {
      const headingElements = document.querySelectorAll('main h1, main h2, main h3, main h4, main h5, main h6');
      const extracted: Heading[] = [];
      
      headingElements.forEach((el) => {
        const id = el.id || el.textContent?.toLowerCase().replace(/\s+/g, '-').replace(/[^\w-]/g, '') || '';
        if (id) {
          el.id = id;
          extracted.push({
            id,
            text: el.textContent || '',
            level: parseInt(el.tagName.charAt(1)),
          });
        }
      });
      
      setHeadings(extracted);
    }, 100);
  });

  createEffect(() => {
    const handleScroll = () => {
      const headingElements = document.querySelectorAll('main h1, main h2, main h3, main h4, main h5, main h6');
      let current = '';
      
      headingElements.forEach((el) => {
        const rect = el.getBoundingClientRect();
        if (rect.top <= 100) {
          current = el.id;
        }
      });
      
      setActiveId(current);
    };

    window.addEventListener('scroll', handleScroll);
    handleScroll();
    
    return () => window.removeEventListener('scroll', handleScroll);
  });

  const scrollToHeading = (id: string) => {
    const element = document.getElementById(id);
    if (element) {
      const offset = 120;
      const elementPosition = element.getBoundingClientRect().top;
      const offsetPosition = elementPosition + window.pageYOffset - offset;
      window.scrollTo({
        top: offsetPosition,
        behavior: 'smooth',
      });
      setActiveId(id);
    }
  };

  return (
    <aside
      class="hidden xl:block w-64 bg-white border-l border-[#e5e3df] overflow-y-auto custom-scrollbar h-[calc(100vh-112px)] sticky top-[112px]"
      id="table-of-contents"
      role="complementary"
      aria-label="Table of contents"
    >
      <Show when={headings().length > 0}>
        <nav class="p-5">
          <h2 class="text-xs font-semibold text-[#6b7280] uppercase tracking-wider mb-3">
            On This Page
          </h2>
          <ul class="space-y-1">
            <For each={headings()}>
              {(heading) => (
                <li>
                  <button
                    onClick={() => scrollToHeading(heading.id)}
                    class={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                      activeId() === heading.id
                        ? 'bg-[#e8f0e9] text-[#2d4a2f] font-medium border-l-2 border-[#4a5a4c]'
                        : 'text-[#4a5568] hover:bg-[#f7f6f4] hover:text-[#2d3748]'
                    }`}
                    style={`padding-left: ${(heading.level - 1) * 0.75 + 0.75}rem;`}
                    aria-current={activeId() === heading.id ? 'location' : undefined}
                  >
                    {heading.text}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </nav>
      </Show>
    </aside>
  );
};

export default TableOfContents;

