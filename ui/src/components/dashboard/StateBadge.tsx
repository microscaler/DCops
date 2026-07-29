// ============================================================================
// StateBadge — colored indicator for CR reconciliation state
// ============================================================================

import { Component } from 'solid-js';
import type { ResourceState } from '../../api/types';

interface StateBadgeProps {
  state: ResourceState | string;
}

const STATE_STYLES: Record<string, { bg: string; text: string; ring: string }> = {
  Created: { bg: 'bg-green-100', text: 'text-green-800', ring: 'ring-green-300' },
  Updated: { bg: 'bg-blue-100', text: 'text-blue-800', ring: 'ring-blue-300' },
  Pending: { bg: 'bg-yellow-100', text: 'text-yellow-800', ring: 'ring-yellow-300' },
  Failed: { bg: 'bg-red-100', text: 'text-red-800', ring: 'ring-red-300' },
};

const DEFAULT_STYLE = { bg: 'bg-gray-100', text: 'text-gray-800', ring: 'ring-gray-300' };

const StateBadge: Component<StateBadgeProps> = (props) => {
  const style = STATE_STYLES[props.state] ?? DEFAULT_STYLE;

  return (
    <span
      class={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset ${style.bg} ${style.text} ${style.ring}`}
    >
      {props.state}
    </span>
  );
};

export default StateBadge;
