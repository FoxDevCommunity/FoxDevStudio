import { Button, Tab, TabList } from '@fluentui/react-components';
import { DismissRegular } from '@fluentui/react-icons';
import { docTitle, useDocumentsStore, type OpenDocument } from '../stores/documentsStore';
import { closeDocument } from '../stores/fileActions';
import { useDocDirty } from './activeDocument';

/** One tab per open document; dirty ones show a trailing asterisk. */
export function DocumentTabs() {
  const docs = useDocumentsStore((s) => s.docs);
  const order = useDocumentsStore((s) => s.order);
  const activeId = useDocumentsStore((s) => s.activeId);
  const activate = useDocumentsStore((s) => s.activate);
  if (order.length === 0) return null;
  return (
    <TabList size="small" selectedValue={activeId} onTabSelect={(_e, d) => activate(String(d.value))} style={{ borderBottom: '1px solid var(--colorNeutralStroke2)', overflowX: 'auto' }} aria-label="Open documents">
      {order.map((id) => (
        <DocTab key={id} doc={docs[id]!} />
      ))}
    </TabList>
  );
}

function DocTab({ doc }: { doc: OpenDocument }) {
  const dirty = useDocDirty(doc);
  const docs = useDocumentsStore((s) => s.docs);
  const title = docTitle(doc, docs);
  return (
    <Tab value={doc.id} data-doc-id={doc.id} aria-label={title}>
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
        {title}
        {dirty ? '*' : ''}
        <Button
          as="a"
          role="button"
          appearance="transparent"
          size="small"
          icon={<DismissRegular />}
          aria-label={`Close ${title}`}
          onClick={(e) => {
            e.stopPropagation();
            void closeDocument(doc.id);
          }}
          style={{ minWidth: 0, padding: 0, height: 16 }}
        />
      </span>
    </Tab>
  );
}
