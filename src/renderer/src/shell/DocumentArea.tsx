import { Text } from '@fluentui/react-components';
import type { OpenDocument } from '../stores/documentsStore';
import { FormDesignerDocument } from '../designer/FormDesignerDocument';
import { ProgramDocument } from '../editor/ProgramDocument';
import { MethodEditorDocument } from '../editor/MethodEditorDocument';
import { RuntimeDesktopDocument } from '../runtime/RuntimeDesktop';
import { MenuDesignerDocument } from '../menu-designer/MenuDesignerDocument';
import { TableDocument } from '../data/TableDocument';
import { DocumentTabs } from './DocumentTabs';
import { useActiveDoc } from './activeDocument';

/** Tabs plus the active document, rendered by kind. */
export function DocumentArea() {
  const doc = useActiveDoc();
  return (
    <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }} data-testid="document-area">
      <DocumentTabs />
      <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>{doc ? <ActiveDocument doc={doc} /> : <EmptyArea />}</div>
    </div>
  );
}

function ActiveDocument({ doc }: { doc: OpenDocument }) {
  switch (doc.kind) {
    case 'form':
      return <FormDesignerDocument key={doc.id} docId={doc.id} store={doc.store} />;
    case 'program':
      return <ProgramDocument key={doc.id} docId={doc.id} />;
    case 'method':
      return <MethodEditorDocument key={doc.id} docId={doc.id} />;
    case 'table':
      return <TableDocument key={doc.id} path={doc.path} />;
    case 'desktop':
      return <RuntimeDesktopDocument key={doc.id} />;
    case 'menu':
      return <MenuDesignerDocument key={doc.id} docId={doc.id} store={doc.store} />;
  }
}

function EmptyArea() {
  return (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--colorNeutralForeground3)' }}>
      <Text size={300}>Open a form, menu or program from the Project Explorer.</Text>
    </div>
  );
}
