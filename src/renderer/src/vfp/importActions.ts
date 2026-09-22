/**
 * The File menu entry for importing a Visual FoxPro project: pick a `.pjx`, convert, report
 * what came across and what did not, then open the result.
 */

import { getApi } from '../api/foxdev';
import { useProjectStore } from '../stores/projectStore';
import { useSessionStore } from '../runtime/session';
import { importVfpProject } from './importVfpProject';

/** Imports a project by path and reports the outcome; shared by the dialog and File > Open. */
export async function importVfpProjectPath(path: string): Promise<void> {
  const api = getApi();
  const print = useSessionStore.getState().print;
  try {
    const report = await importVfpProject(path);
    const summary = `Imported ${report.converted.length + report.referenced.length} item(s)${report.skipped.length ? `, skipped ${report.skipped.length}` : ''}.`;
    print({ kind: 'output', text: summary });

    if (report.skipped.length > 0) {
      // the import still produced a project; say plainly what is missing from it
      await api.dialog.message({
        type: 'warning',
        message: summary,
        detail: report.skipped.map((s) => `${s.path}: ${s.reason}`).join('\n'),
        buttons: ['OK'],
      });
    }
    await useProjectStore.getState().openProject(report.projectPath);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    print({ kind: 'error', text: `Import failed. ${message}` });
    await api.dialog.message({ type: 'error', message: 'The project could not be imported.', detail: message, buttons: ['OK'] });
  }
}

export async function importVfpProjectDialog(): Promise<void> {
  const api = getApi();
  const print = useSessionStore.getState().print;

  const path = await api.dialog.openFile({
    title: 'Import Visual FoxPro Project',
    filters: [{ name: 'Visual FoxPro Project', extensions: ['pjx'] }],
  });
  if (!path) return;

  try {
    const report = await importVfpProject(path);
    const summary = `Imported ${report.converted.length + report.referenced.length} item(s)${report.skipped.length ? `, skipped ${report.skipped.length}` : ''}.`;
    print({ kind: 'output', text: summary });

    if (report.skipped.length > 0) {
      // the import still produced a project; say plainly what is missing from it
      await api.dialog.message({
        type: 'warning',
        message: summary,
        detail: report.skipped.map((s) => `${s.path}: ${s.reason}`).join('\n'),
        buttons: ['OK'],
      });
    }
    await useProjectStore.getState().openProject(report.projectPath);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    print({ kind: 'error', text: `Import failed. ${message}` });
    await api.dialog.message({ type: 'error', message: 'The project could not be imported.', detail: message, buttons: ['OK'] });
  }
}
