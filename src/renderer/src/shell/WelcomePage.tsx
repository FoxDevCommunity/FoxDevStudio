import { Button, Text, Title2 } from '@fluentui/react-components';
import { FolderOpenRegular, AddRegular } from '@fluentui/react-icons';
import { useProjectStore } from '../stores/projectStore';
import { runCommand } from './commands/registry';
import logoUrl from '../assets/logo.png';

/** Shown in the document area when no project is open. */
export function WelcomePage() {
  const recent = useProjectStore((s) => s.recent);
  const openProject = useProjectStore((s) => s.openProject);
  const error = useProjectStore((s) => s.error);
  return (
    <div style={{ padding: 32, display: 'flex', flexDirection: 'column', gap: 16, maxWidth: 560 }} data-testid="welcome">
      <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
        <img src={logoUrl} alt="" width={64} height={64} />
        <Title2>Welcome to FoxDev Studio</Title2>
      </div>
      <Text>Create a new project or open an existing one to start designing forms and menus.</Text>
      <div style={{ display: 'flex', gap: 8 }}>
        <Button appearance="primary" icon={<AddRegular />} onClick={() => void runCommand('file.newProject')}>
          New Project
        </Button>
        <Button icon={<FolderOpenRegular />} onClick={() => void runCommand('file.openProject')}>
          Open Project
        </Button>
      </div>
      {error && (
        <Text style={{ color: 'var(--colorPaletteRedForeground1)' }} role="alert">
          {error}
        </Text>
      )}
      {recent.length > 0 && (
        <div>
          <Text weight="semibold">Recent projects</Text>
          <ul style={{ margin: '4px 0', paddingLeft: 20 }}>
            {recent.map((p) => (
              <li key={p}>
                <Button appearance="transparent" size="small" onClick={() => void openProject(p)}>
                  {p}
                </Button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
