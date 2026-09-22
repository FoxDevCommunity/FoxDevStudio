import { screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import { useProjectStore } from '@renderer/stores/projectStore';
import { useSessionStore } from '@renderer/runtime/session';
import { pictureProblem, pictureUrl } from '@renderer/runtime/pictureUrl';
import { Desktop } from '@shared/runtime/objectModel';
import { RuntimeControl } from '@renderer/runtime/RuntimeControl';
import type { FormDocument } from '@shared/form/schema';
import { renderWithProviders } from '../helpers/render';

function formWith(picture: string): FormDocument {
  return {
    $schema: 'foxdev-form',
    version: 1,
    form: {
      name: 'Form1',
      props: {},
      methods: {},
      children: [{ id: 'img1', type: 'Image', name: 'Image1', props: { Picture: picture, Left: 0, Top: 0, Width: 100, Height: 100 }, methods: {} }],
    },
  };
}

const imageOf = (picture: string) => new Desktop().instantiate(formWith(picture).form, -1).children[0]!;

beforeEach(() => {
  setApi(createMemoryApi());
  useProjectStore.getState().close();
  useSessionStore.setState({ output: [] });
});

describe('picture paths', () => {
  it('serves an absolute path over the picture scheme, in either slash style', () => {
    expect(pictureUrl('C:/pics/fox.gif')).toBe('foxpic://local/C%3A%2Fpics%2Ffox.gif');
    expect(pictureUrl(String.raw`C:\pics\fox.gif`)).toBe('foxpic://local/C%3A%2Fpics%2Ffox.gif');
  });

  it('passes through a URL or data URI unchanged', () => {
    expect(pictureUrl('data:image/gif;base64,AA')).toBe('data:image/gif;base64,AA');
    expect(pictureUrl('https://example.com/a.png')).toBe('https://example.com/a.png');
  });

  it('has nothing to resolve a relative path against without a project', () => {
    // this is exactly what HOME(4) + "Gifs\MorphFox.gif" produces when VFP is not installed
    expect(pictureUrl(String.raw`Gifs\MorphFox.gif`)).toBeNull();
    expect(pictureProblem(String.raw`Gifs\MorphFox.gif`)).toMatch(/relative path and no project is open/);
  });

  it('reports a missing file rather than leaving an empty box', () => {
    expect(pictureProblem('C:/pics/missing.gif')).toMatch(/does not exist/);
    expect(pictureProblem('')).toBe('no picture set');
  });

  it('renders an img for a picture that resolves', () => {
    renderWithProviders(<RuntimeControl obj={imageOf('C:/pics/fox.gif')} />);
    expect(screen.getByRole('img')).toHaveAttribute('src', 'foxpic://local/C%3A%2Fpics%2Ffox.gif');
  });

  it('shows the path instead of an img when it cannot resolve', () => {
    renderWithProviders(<RuntimeControl obj={imageOf(String.raw`Gifs\MorphFox.gif`)} />);
    expect(screen.queryByRole('img')).toBeNull();
    expect(screen.getByText(/MorphFox\.gif/)).toBeInTheDocument();
  });

  it('honours Stretch and RotateFlip on the rendered image', () => {
    const image = imageOf('C:/pics/fox.gif');
    image.set('Stretch', 2);
    image.set('RotateFlip', 5);
    renderWithProviders(<RuntimeControl obj={image} />);

    const img = screen.getByRole('img');
    expect(img).toHaveStyle({ objectFit: 'fill' });
    // 5 is one quarter turn plus a horizontal flip
    expect(img.getAttribute('style')).toContain('rotate(90deg)');
    expect(img.getAttribute('style')).toContain('scaleX(-1)');
  });
});
