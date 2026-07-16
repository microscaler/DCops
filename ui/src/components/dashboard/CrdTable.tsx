// ============================================================================
// CrdTable — sortable, filterable, paginated table for CR instances
// ============================================================================

import { Component, For, createSignal, Show } from 'solid-js';
import type { K8sResource, ResourceState } from '../../api/types';
import StateBadge from './StateBadge';

interface CrdTableProps {
  crs: K8sResource[];
  kind: string;
  onRowClick?: (cr: K8sResource) => void;
}

type SortField = 'name' | 'namespace' | 'state' | 'age' | 'error';
type SortDir = 'asc' | 'desc';

const CrdTable: Component<CrdTableProps> = (props) => {
  const [sortField, setSortField] = createSignal<SortField>('name');
  const [sortDir, setSortDir] = createSignal<SortDir>('asc');
  const [stateFilter, setStateFilter] = createSignal<string>('all');
  const [page, setPage] = createSignal(1);
  const PAGE_SIZE = 50;

  const states: ResourceState[] = ['Created', 'Updated', 'Pending', 'Failed'];

  // Filter
  const filtered = () => {
    let result = props.crs;
    const sf = stateFilter();
    if (sf !== 'all') {
      result = result.filter(
        (c) => (c.status as Record<string, unknown>)?.state === sf,
      );
    }
    return result;
  };

  // Sort
  const sorted = () => {
    const data = filtered();
    const field = sortField();
    const dir = sortDir();
    const mul = dir === 'asc' ? 1 : -1;

    return [...data].sort((a, b) => {
      let va: string, vb: string;
      switch (field) {
        case 'name':
          va = a.metadata.name;
          vb = b.metadata.name;
          break;
        case 'namespace':
          va = a.metadata.namespace;
          vb = b.metadata.namespace;
          break;
        case 'state':
          va = (a.status as Record<string, unknown>)?.state as string ?? '';
          vb = (b.status as Record<string, unknown>)?.state as string ?? '';
          break;
        case 'age': {
          const a1 = a.metadata.creationTimestamp ?? '';
          const b1 = b.metadata.creationTimestamp ?? '';
          return mul * (a1.localeCompare(b1));
        }
        case 'error':
          va = (a.status as Record<string, unknown>)?.error ? '1' : '0';
          vb = (b.status as Record<string, unknown>)?.error ? '1' : '0';
          break;
        default:
          return 0;
      }
      return mul * va.localeCompare(vb);
    });
  };

  const paginated = () => {
    const data = sorted();
    const start = (page() - 1) * PAGE_SIZE;
    return data.slice(start, start + PAGE_SIZE);
  };

  const totalPages = () => Math.max(1, Math.ceil(props.crs.length / PAGE_SIZE));

  const handleSort = (field: SortField) => {
    if (sortField() === field) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDir('asc');
    }
    setPage(1);
  };

  const sortIcon = (field: SortField) => {
    if (sortField() !== field) return '↕';
    return sortDir() === 'asc' ? '↑' : '↓';
  };

  const age = (ts: string) => {
    if (!ts) return '—';
    const diff = Date.now() - new Date(ts).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  };

  return (
    <div class="overflow-hidden rounded-lg border border-gray-200 bg-white">
      {/* Toolbar */}
      <div class="flex items-center justify-between border-b border-gray-200 px-4 py-3">
        <span class="text-sm font-medium text-gray-500">
          {props.crs.length} total
        </span>
        <select
          value={stateFilter()}
          onChange={(e) => { setStateFilter(e.currentTarget.value); setPage(1); }}
          class="rounded-md border-gray-300 text-sm focus:border-indigo-500 focus:ring-indigo-500"
        >
          <option value="all">All states</option>
          <For each={states}>{(s) => <option value={s}>{s}</option>}</For>
        </select>
      </div>

      {/* Table */}
      <Show when={props.crs.length > 0} fallback={
        <div class="flex flex-col items-center justify-center py-16 text-gray-400">
          <svg class="mb-3 h-12 w-12" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
          </svg>
          <p class="text-sm">No {props.kind} resources found</p>
        </div>
      }>
        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm">
            <thead class="border-b border-gray-200 bg-gray-50">
              <tr>
                <th
                  class="cursor-pointer px-4 py-3 font-medium text-gray-600 hover:text-gray-900"
                  onClick={() => handleSort('name')}
                >
                  {sortIcon('name')} Name
                </th>
                <th
                  class="cursor-pointer px-4 py-3 font-medium text-gray-600 hover:text-gray-900"
                  onClick={() => handleSort('namespace')}
                >
                  {sortIcon('namespace')} Namespace
                </th>
                <th
                  class="cursor-pointer px-4 py-3 font-medium text-gray-600 hover:text-gray-900"
                  onClick={() => handleSort('state')}
                >
                  {sortIcon('state')} State
                </th>
                <th
                  class="cursor-pointer px-4 py-3 font-medium text-gray-600 hover:text-gray-900"
                  onClick={() => handleSort('age')}
                >
                  {sortIcon('age')} Age
                </th>
                <th
                  class="cursor-pointer px-4 py-3 font-medium text-gray-600 hover:text-gray-900"
                  onClick={() => handleSort('error')}
                >
                  {sortIcon('error')} Error
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100">
              <For each={paginated()}>
                {(cr) => {
                  const state = (cr.status as Record<string, unknown>)?.state as string ?? '—';
                  const error = (cr.status as Record<string, unknown>)?.error as string | undefined;
                  const isFailed = state === 'Failed';
                  return (
                    <tr
                      class={`cursor-pointer transition-colors ${
                        isFailed
                          ? 'bg-red-50 hover:bg-red-100'
                          : 'hover:bg-gray-50'
                      }`}
                      onClick={() => props.onRowClick?.(cr)}
                      role="button"
                      tabIndex={0}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          props.onRowClick?.(cr);
                        }
                      }}
                    >
                      <td class="px-4 py-2.5 font-medium text-gray-900">
                        {cr.metadata.name}
                      </td>
                      <td class="px-4 py-2.5 text-gray-500">
                        {cr.metadata.namespace}
                      </td>
                      <td class="px-4 py-2.5">
                        <StateBadge state={state} />
                      </td>
                      <td class="px-4 py-2.5 text-gray-500">
                        {age(cr.metadata.creationTimestamp)}
                      </td>
                      <td class="px-4 py-2.5">
                        <Show when={error}>
                          <span class="text-xs text-red-600">
                            {typeof error === 'string' ? error.slice(0, 60) : '—'}
                            {error && typeof error === 'string' && error.length > 60 ? '…' : ''}
                          </span>
                        </Show>
                      </td>
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        </div>
      </Show>

      {/* Pagination */}
      <Show when={totalPages() > 1}>
        <div class="flex items-center justify-between border-t border-gray-200 px-4 py-3">
          <span class="text-sm text-gray-500">
            Page {page()} of {totalPages()}
          </span>
          <div class="flex gap-1">
            <button
              onClick={() => setPage(Math.max(1, page() - 1))}
              disabled={page() === 1}
              class="rounded border border-gray-300 px-3 py-1 text-sm disabled:opacity-50 hover:bg-gray-50"
            >
              ←
            </button>
            <button
              onClick={() => setPage(Math.min(totalPages(), page() + 1))}
              disabled={page() === totalPages()}
              class="rounded border border-gray-300 px-3 py-1 text-sm disabled:opacity-50 hover:bg-gray-50"
            >
              →
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default CrdTable;
