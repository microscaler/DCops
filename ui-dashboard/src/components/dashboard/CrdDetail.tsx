// ============================================================================
// CrdDetail — single CR detail panel with spec/status and NetBox link
// ============================================================================

import { Component, Show } from 'solid-js';
import type { K8sResource } from '../../api/types';
import JsonViewer from '../common/JsonViewer';

interface CrdDetailProps {
  cr: K8sResource;
  onClose: () => void;
}

const CrdDetail: Component<CrdDetailProps> = (props) => {
  const state = () => (props.cr.status as Record<string, unknown>)?.state as string ?? '—';
  const error = () => (props.cr.status as Record<string, unknown>)?.error as string | undefined;
  const netboxUrl = () => (props.cr.status as Record<string, unknown>)?.netboxUrl as string | undefined;
  const netboxId = () => (props.cr.status as Record<string, unknown>)?.netboxId as number | undefined;

  const metadataObj = () => ({
    name: props.cr.metadata.name,
    namespace: props.cr.metadata.namespace,
    creationTimestamp: props.cr.metadata.creationTimestamp,
    labels: props.cr.metadata.labels ?? {},
    annotations: props.cr.metadata.annotations ?? {},
    uid: props.cr.metadata.uid,
    resourceVersion: props.cr.metadata.resourceVersion,
  });

  const statusObj = () => (props.cr.status as Record<string, unknown>) ?? {};

  return (
    <div class="fixed inset-0 z-50 flex justify-end" role="dialog" aria-modal="true">
      {/* Backdrop */}
      <div class="absolute inset-0 bg-black bg-opacity-30" onClick={props.onClose} />

      {/* Panel */}
      <div class="relative w-full max-w-3xl overflow-y-auto bg-white shadow-xl">
        {/* Header */}
        <div class="sticky top-0 z-10 flex items-center justify-between border-b border-gray-200 bg-white px-6 py-4">
          <div>
            <h2 class="text-lg font-semibold text-gray-900">
              {props.cr.kind} · {props.cr.metadata.name}
            </h2>
            <div class="mt-1 flex items-center gap-3 text-sm text-gray-500">
              <span>{props.cr.metadata.namespace}</span>
              <span>·</span>
              <Show when={state() !== '—'}>
                <span class={`font-medium ${state() === 'Failed' ? 'text-red-600' : 'text-green-600'}`}>
                  {state()}
                </span>
              </Show>
            </div>
          </div>
          <button
            onClick={props.onClose}
            class="rounded-lg p-2 text-gray-400 hover:bg-gray-100 hover:text-gray-600"
            aria-label="Close"
          >
            <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div class="px-6 py-6 space-y-6">
          {/* NetBox link */}
          <Show when={netboxUrl()}>
            <div class="rounded-lg border border-indigo-200 bg-indigo-50 p-4">
              <div class="flex items-center gap-2">
                <svg class="h-5 w-5 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
                <span class="text-sm font-medium text-indigo-900">
                  NetBox Resource
                </span>
                {netboxId() != null && (
                  <span class="text-xs text-indigo-600">
                    (ID: {netboxId()})
                  </span>
                )}
              </div>
              <a
                href={netboxUrl()!}
                target="_blank"
                rel="noopener noreferrer"
                class="mt-2 block text-sm text-indigo-600 hover:underline"
              >
                {netboxUrl()}
              </a>
            </div>
          </Show>

          {/* Error */}
          <Show when={error()}>
            <div class="rounded-lg border border-red-200 bg-red-50 p-4">
              <div class="flex items-center gap-2">
                <svg class="h-5 w-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-sm font-medium text-red-900">
                  Reconciliation Error
                </span>
              </div>
              <p class="mt-1 text-sm text-red-700">{error()}</p>
            </div>
          </Show>

          {/* Metadata */}
          <JsonViewer
            data={metadataObj()}
            title="Metadata"
            collapsible
            defaultCollapsed
          />

          {/* Spec */}
          <JsonViewer
            data={props.cr.spec}
            title="Spec"
            collapsible
            defaultCollapsed
          />

          {/* Status */}
          <JsonViewer
            data={statusObj()}
            title="Status"
            collapsible
            defaultCollapsed
          />
        </div>
      </div>
    </div>
  );
};

export default CrdDetail;
