import type { FormDesignerStore } from './store/createFormDesignerStore';
import { FormDesignerProvider } from './store/FormDesignerContext';
import { Toolbox } from './toolbox/Toolbox';
import { DesignerCanvas } from './canvas/DesignerCanvas';
import './designer.css';

/** One open form: toolbox strip plus the design canvas. The properties window lives in the IDE shell. */
export function FormDesignerDocument({ docId, store }: { docId: string; store: FormDesignerStore }) {
  return (
    <FormDesignerProvider store={store} docId={docId}>
      <div className="fx-designer" data-testid="form-designer">
        <Toolbox />
        <DesignerCanvas />
      </div>
    </FormDesignerProvider>
  );
}
