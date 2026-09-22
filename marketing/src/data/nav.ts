/** The documentation, in the order the sidebar lists it. One entry per page. */
export interface DocPage {
  title: string;
  href: string;
  /** A line for the docs index and the sidebar's title attribute. */
  summary: string;
}

export interface DocGroup {
  label: string;
  pages: DocPage[];
}

export const docs: DocGroup[] = [
  {
    label: 'Start here',
    pages: [
      { title: 'Overview', href: '/docs', summary: 'What FoxDev Studio is, and where each part of it is described.' },
      {
        title: 'Getting started',
        href: '/docs/getting-started',
        summary: 'Install the IDE, open a project, run a form, ship an executable.',
      },
      {
        title: 'Required tooling',
        href: '/docs/tooling',
        summary: 'What you need to run the IDE, and what you need to build it from source.',
      },
    ],
  },
  {
    label: 'The runtime',
    pages: [
      { title: 'The virtual machine', href: '/docs/vm', summary: 'Fibers, host requests, and why the VM never blocks.' },
      { title: 'Bytecode', href: '/docs/bytecode', summary: 'The module format, how a program compiles, and what a frame holds.' },
      {
        title: 'Instruction reference',
        href: '/docs/instructions',
        summary: 'Every instruction with its operands and stack effect, generated from the VM source.',
      },
    ],
  },
  {
    label: 'FoxScript',
    pages: [
      {
        title: 'The new keywords',
        href: '/docs/foxscript',
        summary: 'LAMBDA, ENDLAMBDA, the FoxScript namespace and two new value types.',
      },
      { title: 'The HTTP API', href: '/docs/http-api', summary: 'FoxScript.Http, the Node side of it, and why it is shaped this way.' },
    ],
  },
];

export const allDocs: DocPage[] = docs.flatMap((g) => g.pages);
