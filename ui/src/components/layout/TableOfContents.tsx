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

  const extractHeadings = () => {
    // Look for headings in the markdown content area
    const markdownContent = document.querySelector('main .markdown-content');
    const container = markdownContent || document.querySelector('main');
    
    if (!container) {
      return;
    }
    
    const headingElements = container.querySelectorAll('h1, h2, h3, h4, h5, h6');
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
    
    if (extracted.length > 0) {
      setHeadings(extracted);
    }
  };

  createEffect(() => {
    // Reset headings when content changes
    setHeadings([]);
    
    if (!props.content) {
      return;
    }
    
    let extractTimeout: ReturnType<typeof setTimeout> | null = null;
    let observer: MutationObserver | null = null;
    
    const scheduleExtraction = () => {
      if (extractTimeout) {
        clearTimeout(extractTimeout);
      }
      extractTimeout = setTimeout(() => {
        extractHeadings();
      }, 50);
    };
    
    // Use MutationObserver to watch for DOM changes
    const mainElement = document.querySelector('main');
    if (!mainElement) {
      // Retry mechanism for initial load
      let retries = 0;
      const maxRetries = 20;
      const retryInterval = setInterval(() => {
        retries++;
        const main = document.querySelector('main');
        if (main) {
          clearInterval(retryInterval);
          scheduleExtraction();
          
          // Set up observer once main exists
          observer = new MutationObserver(() => {
            scheduleExtraction();
          });
          observer.observe(main, {
            childList: true,
            subtree: true,
          });
        } else if (retries >= maxRetries) {
          clearInterval(retryInterval);
        }
      }, 100);
      
      return () => {
        clearInterval(retryInterval);
        if (extractTimeout) clearTimeout(extractTimeout);
        if (observer) observer.disconnect();
      };
    }

    // Extract headings immediately if content is already rendered
    scheduleExtraction();

    // Watch for changes to the main element's children
    observer = new MutationObserver(() => {
      scheduleExtraction();
    });

    observer.observe(mainElement, {
      childList: true,
      subtree: true,
    });

    // Fallback timeouts to catch different render timings
    const timeouts: ReturnType<typeof setTimeout>[] = [];
    [100, 300, 600].forEach((delay) => {
      const timeout = setTimeout(() => {
        extractHeadings();
      }, delay);
      timeouts.push(timeout);
    });

    return () => {
      if (observer) observer.disconnect();
      if (extractTimeout) clearTimeout(extractTimeout);
      timeouts.forEach(clearTimeout);
    };
  });

  createEffect(() => {
    const handleScroll = () => {
      const markdownContent = document.querySelector('main .markdown-content');
      const container = markdownContent || document.querySelector('main');
      
      if (!container) {
        return;
      }
      
      const headingElements = container.querySelectorAll('h1, h2, h3, h4, h5, h6');
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

