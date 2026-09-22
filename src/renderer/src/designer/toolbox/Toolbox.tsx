import { Toolbar, ToggleButton, Tooltip } from '@fluentui/react-components';
import { CursorRegular } from '@fluentui/react-icons';
import type { ControlType } from '@shared/form/schema';
import { getDescriptor, getProp, TOOLBOX_TYPES } from '@shared/registry';
import { resolveIcon } from '../icons';
import { useFormDesigner } from '../store/FormDesignerContext';
import { useSettingsStore } from '../../stores/settingsStore';

/** Vertical strip of control tools. Click arms a tool; double-click drops the control below the others. */
export function Toolbox() {
  const tool = useFormDesigner((s) => s.tool);
  const setTool = useFormDesigner((s) => s.setTool);
  const addControl = useFormDesigner((s) => s.addControl);
  const form = useFormDesigner((s) => s.doc.form);
  const gridSize = useSettingsStore((s) => s.gridSize);

  const addAtFreeSpot = (type: ControlType) => {
    const bottom = form.children.reduce((m, c) => Math.max(m, Number(getProp(c, 'Top')) + Number(getProp(c, 'Height'))), 0);
    addControl(type, { left: gridSize, top: bottom ? bottom + gridSize : gridSize });
    setTool(null);
  };

  return (
    <Toolbar vertical size="small" className="fx-toolbox" aria-label="Form controls" data-testid="toolbox">
      <Tooltip content="Select" relationship="label">
        <ToggleButton appearance="subtle" icon={<CursorRegular />} checked={tool === null} onClick={() => setTool(null)} aria-label="Select" />
      </Tooltip>
      {TOOLBOX_TYPES.map((type) => {
        const desc = getDescriptor(type);
        const Icon = resolveIcon(desc.icon);
        return (
          <Tooltip key={type} content={desc.displayName} relationship="label">
            <ToggleButton
              appearance="subtle"
              icon={<Icon />}
              checked={tool === type}
              aria-label={desc.displayName}
              data-tool={type}
              onClick={() => setTool(tool === type ? null : type)}
              onDoubleClick={() => addAtFreeSpot(type)}
            />
          </Tooltip>
        );
      })}
    </Toolbar>
  );
}
