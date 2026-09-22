/**
 * The `_SCREEN` document: where running forms live. Replaces the Milestone-1 preview tab.
 * A modal form dims everything behind it, which is what makes `DO FORM x` with WindowType 1
 * feel modal even though the UI thread never blocks.
 */

import { useEffect, useRef, useState } from 'react';
import type { MenuItem } from '@shared/menu/schema';
import { Text } from '@fluentui/react-components';
import { MenuBars, MenuPreview } from '../menu-designer/MenuPreview';
import { useSessionStore } from './session';
import { FormWindow } from './FormWindow';
import { CharacterScreen } from './CharacterScreen';
import { BrowseWindow } from './BrowseWindow';
import { MemoWindow } from './MemoWindow';

export function RuntimeDesktopDocument() {
  const desktop = useSessionStore((s) => s.desktop);
  const status = useSessionStore((s) => s.status);
  const target = useSessionStore((s) => s.target);
  const menu = useSessionStore((s) => s.menu);
  const shortcut = useSessionStore((s) => s.shortcutMenu);
  const menuSkip = useSessionStore((s) => s.menuSkip);
  const screen = useSessionStore((s) => s.screen);
  const browses = useSessionStore((s) => s.browses);
  const memos = useSessionStore((s) => s.memos);
  // re-render when forms open or close
  useSessionStore((s) => s.revision);

  // SKIP FOR expressions are re-evaluated when a menu is installed, as VFP does on activation
  useEffect(() => {
    if (menu) void useSessionStore.getState().refreshMenuSkip();
  }, [menu]);

  // where a shortcut menu appears: Visual FoxPro puts one at MROW(), MCOL(), which is where
  // the pointer last was over the desktop
  const at = useRef({ x: 0, y: 0 });
  const [shown, setShown] = useState({ x: 0, y: 0 });
  useEffect(() => {
    if (shortcut) setShown(at.current);
  }, [shortcut]);

  const forms = desktop?.visibleForms ?? [];
  const modal = [...forms].reverse().find((f) => f.modal);
  const items = menu ? applySkip(menu.items, menuSkip) : [];

  return (
    <div
      data-testid="runtime-desktop"
      onPointerMove={(e) => {
        const box = e.currentTarget.getBoundingClientRect();
        at.current = { x: e.clientX - box.left, y: e.clientY - box.top };
      }}
      style={{ position: 'relative', flex: 1, minHeight: 0, overflow: 'auto', background: 'var(--colorNeutralBackground2)' }}
    >
      {menu && (
        <div style={{ position: 'sticky', top: 0, zIndex: 30 }}>
          <MenuPreview items={items} ariaLabel="Application menu" onChoose={(_path, item) => void useSessionStore.getState().chooseMenuItem(item)} />
        </div>
      )}
      {screen && <CharacterScreen screen={screen} />}
      {browses.map((browse) => (
        <BrowseWindow key={browse.alias} browse={browse} />
      ))}
      {memos.map((memo) => (
        <MemoWindow key={`${memo.alias}.${memo.field}`} memo={memo} />
      ))}
      {forms.length === 0 && !screen && browses.length === 0 && (
        <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--colorNeutralForeground3)' }}>
          <Text size={300}>
            {status === 'idle' ? 'Nothing is running. Use Program > Run Form, or type a command below.' : `Running ${target ?? ''}...`}
          </Text>
        </div>
      )}

      {shortcut && (
        <>
          {/* clicking away dismisses it and the program carries on, as it does in VFP */}
          <div
            data-testid="shortcut-menu-dismiss"
            onClick={() => shortcut.resolve(null)}
            style={{ position: 'absolute', inset: 0, zIndex: 40 }}
          />
          <div
            role="menu"
            aria-label={`${shortcut.menu.name} shortcut menu`}
            style={{
              position: 'absolute',
              left: shown.x,
              top: shown.y,
              zIndex: 41,
              background: 'var(--colorNeutralBackground1)',
              border: '1px solid var(--colorNeutralStroke1)',
              boxShadow: 'var(--shadow8)',
            }}
          >
            <MenuBars items={applySkip(shortcut.menu.items, menuSkip)} onChoose={(_path, item) => shortcut.resolve(item)} />
          </div>
        </>
      )}

      {modal && <div aria-hidden style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.25)', zIndex: 10 }} />}

      {forms.map((form) => (
        <div key={form.handle} style={{ position: 'relative', zIndex: form === modal ? 20 : 1, width: '100%' }}>
          <FormWindow form={form} />
        </div>
      ))}

    </div>
  );
}

/** Applies each item's SKIP FOR result to the `enabled` flag the menu renderer reads. */
function applySkip(items: MenuItem[], skip: Record<string, boolean>): MenuItem[] {
  return items.map((item) => ({
    ...item,
    enabled: skip[item.id] ? false : item.enabled,
    children: item.children ? applySkip(item.children, skip) : item.children,
  }));
}
