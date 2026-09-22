/**
 * Runtime event bus: the preview publishes VFP events here; milestone 2's runtime subscribes
 * to run compiled method code. Pure TypeScript, no React.
 */
export interface RuntimeEvent {
  form: string;
  /** VFP object name, or the form name for form events. */
  control: string;
  event: string;
  args?: unknown[];
}

export interface RuntimeEventFilter {
  control?: string;
  event?: string;
}

export type Unsubscribe = () => void;

export interface RuntimeEventBus {
  emit(ev: RuntimeEvent): void;
  on(filter: RuntimeEventFilter, handler: (ev: RuntimeEvent) => void): Unsubscribe;
}

export function createRuntimeEventBus(): RuntimeEventBus {
  const handlers = new Set<{ filter: RuntimeEventFilter; handler: (ev: RuntimeEvent) => void }>();
  return {
    emit(ev) {
      for (const h of [...handlers]) {
        if (h.filter.control && h.filter.control.toLowerCase() !== ev.control.toLowerCase()) continue;
        if (h.filter.event && h.filter.event.toLowerCase() !== ev.event.toLowerCase()) continue;
        h.handler(ev);
      }
    },
    on(filter, handler) {
      const entry = { filter, handler };
      handlers.add(entry);
      return () => void handlers.delete(entry);
    },
  };
}
