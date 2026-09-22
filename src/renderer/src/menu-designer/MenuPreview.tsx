import { Menu, MenuButton, MenuDivider, MenuItem, MenuList, MenuPopover, MenuTrigger, Toolbar } from '@fluentui/react-components';
import type { ReactNode } from 'react';
import type { MenuItem as Item } from '@shared/menu/schema';
import { isSeparator } from '@shared/menu/schema';
import { formatHotkey, promptToLabel } from './hotkey';

export interface MenuPreviewProps {
  items: Item[];
  /** Called with the item path (e.g. "File.New") when a command item is chosen. */
  onChoose?(path: string, item: Item): void;
  /** Distinguishes the designer's preview from a running application's menu. */
  ariaLabel?: string;
}

/** Renders a menu document as a working Fluent menu bar, in the designer and at runtime. */
export function MenuPreview({ items, onChoose, ariaLabel = 'Menu preview' }: MenuPreviewProps) {
  return (
    <Toolbar size="small" aria-label={ariaLabel} role="menubar" style={{ border: '1px solid var(--colorNeutralStroke1)', background: 'var(--colorNeutralBackground2)' }}>
      {items.map((pad) => (
        <Pad key={pad.id} pad={pad} onChoose={onChoose} />
      ))}
    </Toolbar>
  );
}

function Pad({ pad, onChoose }: { pad: Item; onChoose?: MenuPreviewProps['onChoose'] }) {
  const { label } = promptToLabel(pad.prompt);
  const isMenu = pad.result.type === 'submenu' && pad.children && pad.children.length > 0;
  if (!isMenu) {
    return (
      <MenuButton appearance="transparent" size="small" disabled={pad.enabled === false} onClick={() => onChoose?.(label, pad)} menuIcon={null}>
        <Mnemonic prompt={pad.prompt} />
      </MenuButton>
    );
  }
  return (
    <Menu>
      <MenuTrigger disableButtonEnhancement>
        <MenuButton appearance="transparent" size="small" disabled={pad.enabled === false}>
          <Mnemonic prompt={pad.prompt} />
        </MenuButton>
      </MenuTrigger>
      <MenuPopover>
        <MenuList>{renderBars(pad.children!, label, onChoose)}</MenuList>
      </MenuPopover>
    </Menu>
  );
}

/**
 * The bars of one popup, on their own.
 *
 * A shortcut menu has no bar along the top: Visual FoxPro defines it as a popup and activates it
 * where the pointer is. Its items are the same bars a pad would drop down, so they are rendered
 * by the same code.
 */
export function MenuBars({ items, onChoose }: { items: Item[]; onChoose?: MenuPreviewProps['onChoose'] }) {
  return <MenuList>{renderBars(items, '', onChoose)}</MenuList>;
}

function renderBars(bars: Item[], path: string, onChoose?: MenuPreviewProps['onChoose']): ReactNode {
  return bars.map((bar) => {
    if (isSeparator(bar)) return <MenuDivider key={bar.id} />;
    const { label } = promptToLabel(bar.prompt);
    const full = `${path}.${label}`;
    if (bar.result.type === 'submenu' && bar.children && bar.children.length > 0) {
      return (
        <Menu key={bar.id}>
          <MenuTrigger disableButtonEnhancement>
            <MenuItem disabled={bar.enabled === false}>
              <Mnemonic prompt={bar.prompt} />
            </MenuItem>
          </MenuTrigger>
          <MenuPopover>
            <MenuList>{renderBars(bar.children, full, onChoose)}</MenuList>
          </MenuPopover>
        </Menu>
      );
    }
    return (
      <MenuItem key={bar.id} disabled={bar.enabled === false} secondaryContent={formatHotkey(bar.hotkey)} onClick={() => onChoose?.(full, bar)}>
        <Mnemonic prompt={bar.prompt} />
      </MenuItem>
    );
  });
}

/** Label with the VFP hotkey letter underlined. */
export function Mnemonic({ prompt }: { prompt: string }) {
  const { label, index } = promptToLabel(prompt);
  if (index < 0) return <>{label}</>;
  return (
    <>
      {label.slice(0, index)}
      <u>{label[index]}</u>
      {label.slice(index + 1)}
    </>
  );
}
