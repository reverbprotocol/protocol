import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Reverb Protocol',
  description: 'Substrate library for dispute-mediated commerce on Arc. Interfaces, reference implementations, and the deployed UUPS proxy on Arc testnet.',
  base: '/protocol/',
  cleanUrls: true,
  appearance: 'dark',
  ignoreDeadLinks: true,
  themeConfig: {
    nav: [
      { text: 'Overview', link: '/' },
      { text: 'Interfaces', link: '/interfaces/' },
      { text: 'Reference', link: '/reference/' },
      { text: 'CHANGELOG', link: '/changelog' },
      { text: 'GitHub', link: 'https://github.com/reverbprotocol/protocol' },
    ],
    sidebar: [
      {
        text: 'Start',
        items: [
          { text: 'Overview', link: '/' },
        ],
      },
      {
        text: 'Substrate primitives',
        items: [
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
