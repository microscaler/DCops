import { Component, createEffect, createSignal, onCleanup } from 'solid-js';
import { marked } from 'marked';
import mermaid from 'mermaid';

interface MarkdownRendererProps {
  content: string;
}

const MarkdownRenderer: Component<MarkdownRendererProps> = (props) => {
  const [html, setHtml] = createSignal<string>('');
  let containerRef: HTMLDivElement | undefined;
  let mermaidTimeout: ReturnType<typeof setTimeout> | null = null;

  createEffect(() => {
    if (!props.content) {
      setHtml('');
      return;
    }

    if (mermaidTimeout) {
      clearTimeout(mermaidTimeout);
      mermaidTimeout = null;
    }

    marked.setOptions({
      breaks: true,
      gfm: true,
    });

    const rendered = marked.parse(props.content);
    setHtml(rendered as string);
    
    mermaidTimeout = setTimeout(() => {
      const container = containerRef;
      if (container) {
        mermaid.initialize({ 
          startOnLoad: false, 
          theme: 'default',
          securityLevel: 'loose',
        });
        const mermaidElements = container.querySelectorAll('.language-mermaid');
        mermaidElements.forEach((el) => {
          const code = el.textContent || '';
          const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
          mermaid.render(id, code).then((result) => {
            const wrapper = document.createElement('div');
            wrapper.className = 'mermaid';
            wrapper.innerHTML = result.svg;
            
            const svg = wrapper.querySelector('svg');
            if (svg) {
              const rootGroup = svg.querySelector('g:first-child');
              if (rootGroup) {
                const firstRect = rootGroup.querySelector('rect:first-child');
                if (firstRect) {
                  const fill = firstRect.getAttribute('fill');
                  const width = parseFloat(firstRect.getAttribute('width') || '0');
                  const height = parseFloat(firstRect.getAttribute('height') || '0');
                  const x = parseFloat(firstRect.getAttribute('x') || '0');
                  const y = parseFloat(firstRect.getAttribute('y') || '0');
                  
                  if (x === 0 && y === 0 && width > 500 && height > 300 && 
                      (fill === '#1f2328' || fill === '#0d1117' || fill === '#161b22' || 
                       fill === '#21262d' || fill === '#000000')) {
                    firstRect.setAttribute('fill', '#ffffff');
                    firstRect.setAttribute('stroke', 'none');
                  }
                }
              }
            }
            
            el.parentNode?.replaceChild(wrapper, el);
          }).catch((err) => {
            console.error('Mermaid rendering error:', err);
          });
        });
      }
      mermaidTimeout = null;
    }, 100);
  });

  onCleanup(() => {
    if (mermaidTimeout) {
      clearTimeout(mermaidTimeout);
    }
  });

  return (
    <div
      ref={containerRef}
      innerHTML={html()}
      class="markdown-content"
    />
  );
};

export default MarkdownRenderer;

