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
              // Set SVG background color
              svg.style.backgroundColor = '#e1f5ff';
              
              // Find and replace all dark background rectangles
              const allRects = svg.querySelectorAll('rect');
              allRects.forEach((rect) => {
                const fill = rect.getAttribute('fill') || '';
                const style = rect.getAttribute('style') || '';
                const computedStyle = window.getComputedStyle(rect);
                const actualFill = fill || computedStyle.fill || '';
                
                // Check if this is a background rectangle (large, at origin, dark color)
                const width = parseFloat(rect.getAttribute('width') || '0');
                const height = parseFloat(rect.getAttribute('height') || '0');
                const x = parseFloat(rect.getAttribute('x') || '0');
                const y = parseFloat(rect.getAttribute('y') || '0');
                
                // Dark colors to replace
                const darkColors = [
                  '#1f2328', '#0d1117', '#161b22', '#21262d', '#000000',
                  '#1a1a1a', '#2d2d2d', '#333333', '#1e1e1e', '#0a0a0a'
                ];
                
                const isDarkBackground = darkColors.some(color => 
                  actualFill.toLowerCase() === color.toLowerCase() ||
                  actualFill.toLowerCase() === color.toLowerCase().replace('#', '')
                );
                
                // If it's a large rectangle at the origin with dark fill, replace it
                if (isDarkBackground && width > 200 && height > 100 && 
                    Math.abs(x) < 10 && Math.abs(y) < 10) {
                  rect.setAttribute('fill', '#e1f5ff');
                  rect.setAttribute('style', (style + '; fill: #e1f5ff !important;').replace(/fill:[^;]+;?/gi, ''));
                }
              });
            }
            
            // Replace the element
            const parent = el.parentNode;
            if (parent) {
              parent.replaceChild(wrapper, el);
              
              // If parent is a pre element, add a class to mark it as containing mermaid
              if (parent.tagName === 'PRE') {
                parent.classList.add('contains-mermaid');
              }
            }
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

