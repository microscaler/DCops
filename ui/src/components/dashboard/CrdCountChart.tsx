// ============================================================================
// CrdCountChart — lightweight SVG bar chart of CR distribution by category
// ============================================================================

import { Component, For } from 'solid-js';

interface BarItem {
  category: string;
  count: number;
}

interface CrdCountChartProps {
  data: BarItem[];
  maxCount?: number;
}

const CATEGORY_COLORS: Record<string, string> = {
  ipam: '#10b981',    // emerald
  dcim: '#3b82f6',    // blue
  tenancy: '#8b5cf6', // violet
  extras: '#f59e0b',  // amber
  boot: '#ef4444',    // red
  ippool: '#06b6d4',  // cyan
};

const CrdCountChart: Component<CrdCountChartProps> = (props) => {
  const max = props.maxCount ?? Math.max(1, ...props.data.map((d) => d.count));

  return (
    <div class="rounded-lg border border-gray-200 bg-white p-4">
      <h3 class="mb-3 text-sm font-semibold text-gray-600 uppercase tracking-wider">
        CRs by Category
      </h3>
      <div class="space-y-2">
        <For each={props.data.sort((a, b) => b.count - a.count)}>
          {(item) => (
            <div class="flex items-center gap-3">
              <span class="w-20 text-right text-sm font-medium text-gray-700">
                {item.category}
              </span>
              <div class="flex-1 overflow-hidden rounded-full bg-gray-100">
                <div
                  class="h-5 rounded-full transition-all duration-500"
                  style={{
                    width: `${(item.count / max) * 100}%`,
                    'background-color': CATEGORY_COLORS[item.category] ?? '#6b7280',
                  }}
                />
              </div>
              <span class="w-8 text-right text-sm font-mono text-gray-600">
                {item.count}
              </span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

export default CrdCountChart;
