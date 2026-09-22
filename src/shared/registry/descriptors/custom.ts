/**
 * The Visual FoxPro base classes a form carries that are not controls.
 *
 * A real VFP form is full of these. Every Solution sample holds a `C_solutions1` (a `custom`),
 * and the Foundation Classes are almost entirely `custom` objects that a form drops on itself to
 * borrow behaviour from. They have properties and methods and no appearance, so they are modelled
 * the way Timer already is: non-visual, drawn as an icon in the designer, absent at run time.
 *
 * Importing them matters more than showing them. A `custom` dropped on the floor takes its
 * methods with it, and those methods are the point of the object.
 */

import type { ControlDescriptor } from '../types';
import * as c from '../common';

/** Properties every non-visual object has: a position in the designer, and the usual bookkeeping. */
const nonVisualProps = c.props(
  c.prop('Left', 'number', 'Layout', 0),
  c.prop('Top', 'number', 'Layout', 0),
  c.prop('Comment', 'multiline', 'Other', ''),
  c.prop('Tag', 'text', 'Other', ''),
);

export const custom: ControlDescriptor = {
  type: 'Custom',
  baseClass: 'custom',
  displayName: 'Custom',
  toolboxGroup: 'Other',
  icon: 'Cube',
  namePrefix: 'Custom',
  nonVisual: true,
  defaultSize: { Width: 24, Height: 24 },
  defaultEvent: 'Init',
  // VFP lets a custom hold other objects; the Foundation Classes lean on that heavily
  container: { accepts: 'visual' },
  properties: nonVisualProps,
  events: c.events(c.baseEvents),
};

/**
 * `session` is a `custom` with its own private data session. Nothing here has data sessions yet,
 * so it imports as an object that keeps its properties and methods and says nothing more.
 */
export const session: ControlDescriptor = {
  type: 'Session',
  baseClass: 'session',
  displayName: 'Session',
  toolboxGroup: 'Other',
  icon: 'Cube',
  namePrefix: 'Session',
  nonVisual: true,
  defaultSize: { Width: 24, Height: 24 },
  defaultEvent: 'Init',
  properties: nonVisualProps,
  events: c.events(c.baseEvents),
};

/**
 * VFP's `collection`: a keyed list of things, held as a member like any other object. The
 * Foundation Classes are full of them - a web service class keeps its parameters in one.
 *
 * Asked what it is, a real Collection answers with `Comment`, `KeySort`, `Name` and `Tag` and
 * nothing else that can be set: no Height, no Width, no AddObject. It cannot hold controls, so
 * it has no `container`. `Left` and `Top` come with the other non-visual objects because that
 * is where the class designer parks the icon it draws for one.
 */
export const collection: ControlDescriptor = {
  type: 'Collection',
  baseClass: 'collection',
  displayName: 'Collection',
  toolboxGroup: 'Other',
  icon: 'AppsList',
  namePrefix: 'Collection',
  nonVisual: true,
  defaultSize: { Width: 24, Height: 24 },
  defaultEvent: 'Init',
  properties: c.props(
    nonVisualProps,
    // 0 leaves the items in the order they were added, 1 sorts by key ascending, 2 descending
    c.prop('KeySort', 'number', 'Other', 0),
  ),
  events: c.events(c.baseEvents),
};

/** VFP's `hyperlink` object: it opens a URL and has no appearance of its own. */
export const hyperlink: ControlDescriptor = {
  type: 'Hyperlink',
  baseClass: 'hyperlink',
  displayName: 'Hyperlink',
  toolboxGroup: 'Other',
  icon: 'Link',
  namePrefix: 'Hyperlink',
  nonVisual: true,
  defaultSize: { Width: 24, Height: 24 },
  defaultEvent: 'Init',
  properties: nonVisualProps,
  events: c.events(c.baseEvents),
};
