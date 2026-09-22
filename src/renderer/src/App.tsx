import { useEffect } from 'react';
import { FluentProvider, webDarkTheme, webLightTheme } from '@fluentui/react-components';
import { getApi } from './api/foxdev';
import { IdeLayout } from './shell/IdeLayout';
import { useSettingsStore } from './stores/settingsStore';
import { useProjectStore } from './stores/projectStore';
import { closeProject, openProjectItem } from './stores/fileActions';
import { useAnyDirty } from './shell/activeDocument';
import { loadFoxVm } from '../../wasm/foxvm/loader';

export function App() {
  const theme = useSettingsStore((s) => s.theme);
  const projectName = useProjectStore((s) => s.doc?.name);
  const dirty = useAnyDirty();

  useEffect(() => {
    const api = getApi();
    void useProjectStore
      .getState()
      .loadRecent()
      .then(() => api.app.getStartupProject())
      .then(async (p) => {
        if (!p || !(await useProjectStore.getState().openProject(p))) return;
        const main = useProjectStore.getState().doc?.main;
        if (main) await openProjectItem(main).catch(() => undefined);
      });
    const offClose = api.app.onCloseRequested(() => {
      void closeProject().then((ok) => api.app.confirmClose(ok));
    });
    const onExit = () => void closeProject().then((ok) => api.app.confirmClose(ok));
    window.addEventListener('foxdev:exit', onExit);
    return () => {
      offClose();
      window.removeEventListener('foxdev:exit', onExit);
    };
  }, []);

  useEffect(() => {
    void loadFoxVm()
      .then((vm) => console.log(`[foxdev] foxvm ${vm.version()} loaded`))
      .catch((err: unknown) => console.error('[foxdev] foxvm failed to load', err));
  }, []);

  useEffect(() => {
    void getApi().app.setTitle(`${projectName ? projectName + ' - ' : ''}FoxDev Studio`);
  }, [projectName]);

  useEffect(() => {
    void getApi().app.setDocumentEdited(dirty);
  }, [dirty]);

  return (
    <FluentProvider theme={theme === 'dark' ? webDarkTheme : webLightTheme} style={{ height: '100%' }}>
      <IdeLayout />
    </FluentProvider>
  );
}
