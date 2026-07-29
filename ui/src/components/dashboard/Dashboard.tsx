// ============================================================================
// Dashboard — main dashboard page
// ============================================================================

import { Component, createSignal, createEffect, For, Show } from 'solid-js';
import { K8sClient } from '../../api/k8s-client';
import { fetchDashboardData } from '../../api/crd-discovery';
import type { DashboardData, K8sResource } from '../../api/types';
import CrdTable from './CrdTable';
import CrdDetail from './CrdDetail';
import NamespaceFilter from './NamespaceFilter';
import ErrorSummary from './ErrorSummary';
import CrdCountChart from './CrdCountChart';

const Dashboard: Component = () => {
  // Determine API endpoint based on environment
  const isLocal = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';
  const client = new K8sClient(isLocal ? 'http://localhost:8001' : '');

  const [data, setData] = createSignal<DashboardData | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [selectedNs, setSelectedNs] = createSignal('all');
  const [selectedCrd, setSelectedCrd] = createSignal<string | null>(null);
  const [selectedCr, setSelectedCr] = createSignal<K8sResource | null>(null);
  const [autoRefresh, setAutoRefresh] = createSignal(true);

  // Fetch data
  const fetchData = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await fetchDashboardData(client);
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch data');
    } finally {
      setLoading(false);
    }
  };

  // Initial fetch
  createEffect(() => {
    fetchData();
  });

  // Auto-refresh every 30s
  createEffect(() => {
    if (!autoRefresh()) return;
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  });

  // Build chart data from categories
  const chartData = () => {
    const d = data();
    if (!d) return [];
    // summary.byCategory is already the correct aggregate counts
    return Object.entries(d.summary.byCategory).map(
      ([category, count]) => ({ category, count }),
    );
  };

  // Get filtered CR instances
  const filteredInstances = () => {
    const d = data();
    if (!d || !selectedCrd()) return [];
    return d.instances.get(selectedCrd()!) ?? [];
  };

  // Refresh handler
  const handleRefresh = () => {
    fetchData();
  };

  // Row click handler
  const handleRowClick = (cr: K8sResource) => {
    setSelectedCr(cr);
  };

  // Close detail handler
  const handleCloseDetail = () => {
    setSelectedCr(null);
  };

  return (
    <div class="flex flex-col gap-4 p-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900">Dashboard</h1>
          <p class="mt-1 text-sm text-gray-500">
            View and manage {data()?.summary.total ?? '—'} resources across
            {data()?.namespaces.length ?? '—'} namespaces
          </p>
        </div>
        <div class="flex items-center gap-3">
          <label class="flex items-center gap-2 text-sm text-gray-600">
            <input
              type="checkbox"
              checked={autoRefresh()}
              onChange={(e) => setAutoRefresh(e.currentTarget.checked)}
              class="rounded border-gray-300 text-indigo-600"
            />
            Auto-refresh
          </label>
          <button
            onClick={handleRefresh}
            class="inline-flex items-center gap-1 rounded-lg bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            Refresh
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      <Show when={!loading() && data()}>
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-4">
          <div class="rounded-lg border border-gray-200 bg-white p-4">
            <dt class="text-sm font-medium text-gray-500">Total CRs</dt>
            <dd class="mt-1 text-3xl font-bold text-gray-900">
              {data()?.summary.total ?? '—'}
            </dd>
          </div>
          <div class="rounded-lg border border-gray-200 bg-white p-4">
            <dt class="text-sm font-medium text-gray-500">Created</dt>
            <dd class="mt-1 text-3xl font-bold text-green-600">
              {data()?.summary.byState['Created'] ?? '—'}
            </dd>
          </div>
          <div class="rounded-lg border border-gray-200 bg-white p-4">
            <dt class="text-sm font-medium text-gray-500">Failed</dt>
            <dd class="mt-1 text-3xl font-bold text-red-600">
              {data()?.summary.withErrors ?? '—'}
            </dd>
          </div>
          <div class="rounded-lg border border-gray-200 bg-white p-4">
            <dt class="text-sm font-medium text-gray-500">Namespaces</dt>
            <dd class="mt-1 text-3xl font-bold text-gray-900">
              {data()?.namespaces.length ?? '—'}
            </dd>
          </div>
        </div>

        {/* Chart + Error Summary */}
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <CrdCountChart
            data={Object.entries(chartData()).map(([category, count]) => ({ category, count }))}
          />
          <ErrorSummary crs={filteredInstances()} />
        </div>

        {/* Filter row */}
        <div class="flex items-center gap-3">
          <NamespaceFilter
            namespaces={data()!.namespaces}
            selected={selectedNs()}
            onChange={(ns) => { setSelectedNs(ns); setSelectedCrd(null); }}
          />

          <select
            value={selectedCrd() ?? ''}
            onChange={(e) => setSelectedCrd(e.currentTarget.value || null)}
            class="rounded-lg border border-gray-300 px-3 py-1.5 text-sm focus:border-indigo-500 focus:ring-indigo-500"
          >
            <option value="">All resource types</option>
            <For each={data()!.crds}>
              {(crd) => (
                <option value={crd.plural}>
                  {crd.kind} ({data()!.instances.get(crd.plural)?.length ?? 0})
                </option>
              )}
            </For>
          </select>
        </div>

        {/* CR Table */}
        <Show when={selectedCrd() && data()}>
          <CrdTable
            crs={filteredInstances()}
            kind={selectedCrd()!}
            onRowClick={handleRowClick}
          />
        </Show>
      </Show>

      {/* Loading state */}
      <Show when={loading()}>
        <div class="flex flex-col items-center justify-center rounded-lg border border-gray-200 bg-white py-20">
          <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600" />
          <p class="mt-4 text-sm text-gray-500">Loading resources...</p>
        </div>
      </Show>

      {/* Error state */}
      <Show when={error()}>
        <div class="rounded-lg border border-red-200 bg-red-50 p-6">
          <div class="flex items-start gap-3">
            <svg class="mt-0.5 h-5 w-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div>
              <h3 class="text-sm font-medium text-red-800">Failed to load data</h3>
              <p class="mt-1 text-sm text-red-700">{error()}</p>
              <button
                onClick={fetchData}
                class="mt-3 rounded bg-red-100 px-3 py-1.5 text-sm font-medium text-red-700 hover:bg-red-200"
              >
                Retry
              </button>
            </div>
          </div>
        </div>
      </Show>

      {/* Detail panel */}
      <Show when={selectedCr()}>
        <CrdDetail cr={selectedCr()!} onClose={handleCloseDetail} />
      </Show>
    </div>
  );
};

export default Dashboard;
