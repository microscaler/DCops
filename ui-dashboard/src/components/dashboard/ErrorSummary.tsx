// ============================================================================
// ErrorSummary — accordion list of CRs with reconciliation errors
// ============================================================================

import { Component, For, Show, createSignal } from 'solid-js';
import type { K8sResource } from '../../api/types';

interface ErrorSummaryProps {
  crs: K8sResource[];
}

const ErrorSummary: Component<ErrorSummaryProps> = (props) => {
  const [open, setOpen] = createSignal(false);

  const errored = props.crs.filter(
    (c) => (c.status as Record<string, unknown>)?.error,
  );

  if (errored.length === 0) return null;

  return (
    <div class="mt-4 rounded-lg border border-red-200 bg-red-50">
      <button
        onClick={() => setOpen(!open())}
        class="flex w-full items-center justify-between px-4 py-3 text-left text-sm font-semibold text-red-800"
        aria-expanded={open()}
      >
        <span>
          {errored.length} resource{errored.length !== 1 ? 's' : ''} with errors
        </span>
        <svg
          class={`h-5 w-5 transition-transform ${open() ? 'rotate-180' : ''}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      <Show when={open()}>
        <ul class="border-t border-red-200 px-4 py-2">
          <For each={errored}>
            {(cr) => (
              <li class="py-2 text-sm">
                <div class="font-medium text-red-900">
                  {cr.kind} · {cr.metadata.namespace} · {cr.metadata.name}
                </div>
                <div class="text-red-700">
                  {(cr.status as Record<string, unknown>)?.error as string}
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
};

export default ErrorSummary;
