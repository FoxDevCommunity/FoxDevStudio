import type { ControlNode, FormNode } from '@shared/form/schema';
import { getObjectDescriptor } from '@shared/registry';
import { FORM_ID } from '../store/createFormDesignerStore';
import { useFormDesignerContext } from '../store/FormDesignerContext';
import { useDocumentsStore } from '../../stores/documentsStore';

/** VFP "Methods" tab: every event of the selected object; double-click opens its code. */
export function MethodsTab({ target }: { target: ControlNode | FormNode }) {
  const { docId } = useFormDesignerContext();
  const desc = getObjectDescriptor(target);
  const id = 'type' in target ? target.id : FORM_ID;
  const open = (method: string) => useDocumentsStore.getState().openMethod(docId, id, method);
  return (
    <table>
      <tbody>
        {desc.events.map((ev) => {
          const src = target.methods[ev.name];
          return (
            <tr key={ev.name} className="fx-event-row" data-event={ev.name} onDoubleClick={() => open(ev.name)} title={ev.params ? `${ev.name}(${ev.params})` : ev.name}>
              <td className={`fx-prop-name${src ? ' fx-changed' : ''}`}>{ev.name}</td>
              <td>{src ? '[User Procedure]' : ev.description ?? ''}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
