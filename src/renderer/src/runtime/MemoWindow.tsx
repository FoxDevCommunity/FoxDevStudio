/**
 * The memo editing window `MODIFY MEMO` opens.
 *
 * Visual FoxPro puts a small text window over the desktop for one memo field, and what is left
 * in it goes back into the field when the window closes. The VM hands the text over and takes
 * back whatever the window was left with, so nothing here reads or writes a table.
 */

import { Button, Text, Textarea } from '@fluentui/react-components';
import { DismissRegular } from '@fluentui/react-icons';
import { useSessionStore, type OpenMemo } from './session';

export function MemoWindow({ memo }: { memo: OpenMemo }) {
  const close = (): void => useSessionStore.getState().closeMemo(memo.alias, memo.field);
  return (
    <div
      data-testid={`memo-${memo.alias}.${memo.field}`}
      style={{
        border: '1px solid var(--colorNeutralStroke1)',
        borderRadius: 4,
        margin: 8,
        background: 'var(--colorNeutralBackground1)',
        boxShadow: 'var(--shadow8)',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '4px 8px',
          background: 'var(--colorNeutralBackground3)',
        }}
      >
        <Text weight="semibold">{`${memo.alias}.${memo.field}`}</Text>
        <span style={{ flex: 1 }} />
        <Button appearance="subtle" size="small" icon={<DismissRegular />} aria-label="Close" onClick={close} />
      </div>
      <Textarea
        aria-label={`${memo.alias}.${memo.field}`}
        value={memo.text}
        readOnly={memo.noedit}
        resize="vertical"
        style={{ width: '100%' }}
        textarea={{ style: { minHeight: 120, fontFamily: 'Consolas, "Cascadia Mono", monospace' } }}
        onChange={(_e, data) => useSessionStore.getState().setMemoText(memo.alias, memo.field, data.value)}
      />
    </div>
  );
}
