import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Reverb Protocol',
  description: 'Substrate library for dispute-mediated commerce on Arc. Interfaces, reference implementations, operating model, and the deployed UUPS proxy on Arc testnet.',
  base: '/protocol/',
  cleanUrls: true,
  appearance: 'dark',
  ignoreDeadLinks: true,
  themeConfig: {
    nav: [
      { text: 'Overview', link: '/' },
      { text: 'Quickstart', link: '/quickstart' },
      { text: 'Architecture', link: '/architecture' },
      { text: 'Autonomy', link: '/AUTONOMY_SPECTRUM' },
      { text: 'Operating model', link: '/OPERATING_MODEL' },
      { text: 'Interfaces', link: '/interfaces/' },
      { text: 'Scenarios', link: '/scenarios/' },
      { text: 'Guides', link: '/guides/build-your-own-forager' },
      { text: 'GitHub', link: 'https://github.com/reverbprotocol/protocol' },
    ],
    sidebar: [
      {
        text: 'Start',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Quickstart', link: '/quickstart' },
          { text: 'Architecture', link: '/architecture' },
          { text: 'Autonomy spectrum', link: '/AUTONOMY_SPECTRUM' },
          { text: 'Operating model', link: '/OPERATING_MODEL' },
        ],
      },
      {
        text: 'Substrate primitives',
        items: [
          { text: 'Overview', link: '/interfaces/' },
          { text: 'IRefundProtocol', link: '/interfaces/IRefundProtocol' },
          { text: 'IBountyAccrual', link: '/interfaces/IBountyAccrual' },
          { text: 'IReputationRegistry', link: '/interfaces/IReputationRegistry' },
          { text: 'ICCTPReceiver', link: '/interfaces/ICCTPReceiver' },
          { text: 'IBondYieldVault', link: '/interfaces/IBondYieldVault' },
          { text: 'IStableFXSwap', link: '/interfaces/IStableFXSwap' },
          { text: 'IAttributable', link: '/interfaces/IAttributable' },
        ],
      },
      {
        text: 'Reference implementations',
        items: [
          { text: 'Overview', link: '/reference/' },
        ],
      },
      {
        text: 'Scenarios',
        items: [
          { text: 'Overview', link: '/scenarios/' },
          { text: 'Dispute via hum', link: '/scenarios/dispute-via-hum' },
          { text: 'Cross-product arbiter', link: '/scenarios/cross-product-arbiter' },
          { text: 'UUPS upgrade lifecycle', link: '/scenarios/uups-upgrade' },
        ],
      },
      {
        text: 'Guides',
        items: [
          { text: 'Build your own forager', link: '/guides/build-your-own-forager' },
          { text: 'Build your own persona', link: '/guides/build-your-own-persona' },
        ],
      },
      {
        text: 'Architecture',
        items: [
          { text: 'HumdRegistry sidecar pattern', link: '/humd-registry-sidecar' },
          { text: 'Security posture', link: '/security-posture' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'CHANGELOG', link: '/changelog' },
          { text: 'FAQ', link: '/faq' },
          { text: 'Glossary', link: '/glossary' },
          { text: 'Source on GitHub', link: 'https://github.com/reverbprotocol/protocol' },
        ],
      },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/reverbprotocol/protocol' },
    ],
    footer: {
      message: 'Apache-2.0.',
      copyright: 'Reverb Protocol — substrate library for dispute-mediated commerce on Arc.',
    },
    search: {
      provider: 'local',
    },
  },
})
