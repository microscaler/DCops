// ============================================================================
// JsonViewer — syntax-highlighted JSON with copy button and collapse/expand
// ============================================================================

import { Component, createSignal, Show } from 'solid-js';

interface JsonViewerProps {
  data: Record<string, unknown>;
  title?: string;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
}

const JsonViewer: Component<JsonViewerProps> = (props) => {
  const [collapsed, setCollapsed] = createSignal(props.defaultCollapsed ?? false);
  const [copied, setCopied] = createSignal(false);

  const jsonString = JSON.stringify(props.data, null, 2);

  const highlightJson = (json: string) => {
    return json.replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        let cls = 'text-orange-600'; // number
        if (/^"/.test(match)) {
          if (/:$/.test(match)) {
            cls = 'text-blue-600'; // key
          } else {
            cls = 'text-green-600'; // string
          }
        } else if (/true|false/.test(match)) {
          cls = 'text-purple-600'; // boolean
        } else if (/null/.test(match)) {
          cls = 'text-gray-500'; // null
        }
        return `<span class="${cls}">${match}</span>`;
      },
    );
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(jsonString);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard not available
    }
  };

  return (
    <div class="rounded-lg border border-gray-200 bg-white">
      {/* Header */}
      <div class="flex items-center justify-between border-b border-gray-200 px-4 py-2">
        <div class="flex items-center gap-2">
          <Show when={props.collapsible}>
            <button
              onClick={() => setCollapsed(!collapsed())}
              class="text-gray-400 hover:text-gray-600"
            >
              {collapsed() ? '▶' : '▼'}
            </button>
          </Show>
          <span class="text-sm font-medium text-gray-700">
            {props.title ?? 'JSON'}
          </span>
        </div>
        <button
          onClick={handleCopy}
          class={`rounded px-2 py-1 text-xs font-medium transition-colors ${
            copied()
              ? 'bg-green-100 text-green-700'
              : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
          }`}
        >
          {copied() ? 'Copied!' : 'Copy'}
        </button>
      </div>

      {/* Content */}
      <Show when={!collapsed()}>
        <div
          class="overflow-x-auto p-4 text-sm"
          innerHTML={highlightJson(jsonString)}
        />
      </Show>
    </div>
  );
};

export default JsonViewer;
