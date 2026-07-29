// ============================================================================
// NamespaceFilter — dropdown to select a namespace for CR filtering
// ============================================================================

import { Component, For, Show } from 'solid-js';
import { createSignal } from 'solid-js';

interface NamespaceFilterProps {
  namespaces: string[];
  selected: string;
  onChange: (ns: string) => void;
}

const NamespaceFilter: Component<NamespaceFilterProps> = (props) => {
  const [open, setOpen] = createSignal(false);

  const allNs = ['all', ...props.namespaces];
  const display = allNs.includes(props.selected) ? props.selected : 'all';

  return (
    <div class="relative">
      <button
        onClick={() => setOpen(!open())}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 bg-white px-3 py-1.5 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
        aria-haspopup="listbox"
        aria-expanded={open()}
      >
        <svg class="h-4 w-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        {display}
      </button>

      <Show when={open()}>
        <>
          <div class="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <ul
            class="absolute right-0 z-20 mt-1 max-h-60 w-56 overflow-auto rounded-lg border border-gray-200 bg-white py-1 shadow-lg"
            role="listbox"
          >
            <For each={allNs}>
              {(ns) => (
                <li>
                  <button
                    onClick={() => {
                      props.onChange(ns);
                      setOpen(false);
                    }}
                    class={`w-full px-4 py-2 text-left text-sm hover:bg-gray-100 ${
                      ns === props.selected ? 'bg-indigo-50 font-semibold text-indigo-700' : 'text-gray-700'
                    }`}
                    role="option"
                    aria-selected={ns === props.selected}
                  >
                    {ns === 'all' ? '(all namespaces)' : ns}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </>
      </Show>
    </div>
  );
};

export default NamespaceFilter;
