import { html, LitElement, PropertyValues } from "lit";
import { customElement, property } from "lit/decorators.js";
import { davClient } from "./dav-client";
import { FileStat } from "webdav";
import { join } from 'path-browserify'


@customElement('file-router')
export class FileRouter extends LitElement {
  constructor() {
    super()
    this.updateRoute()
  }
  protected createRenderRoot(): HTMLElement {
    return this
  }

  @property()
  path: string | null = null

  connectedCallback(): void {
    super.connectedCallback()
    window.addEventListener('routechange', this.updateRoute)
    window.addEventListener('popstate', this._onPopState, false)
  }

  disconnectedCallback(): void {
    super.disconnectedCallback()
    window.removeEventListener('routechange', this.updateRoute)
    window.removeEventListener('popstate', this._onPopState, false)
  }

  updateRoute = () => {
    this.path = window.location.pathname.replace(/^\/frontend/, '')
  }

  _onPopState = (event: PopStateEvent) => {
    window.dispatchEvent(new Event('routechange'))
    event.preventDefault()
  }

  override render() {
    return html`
      <h1>${this.path}</h1>
      <file-browser path=${this.path} />
    `
  }
}

function formatFilesize(size: number): string {
  let order = 0
  let units = ['', 'K', 'M', 'G', 'T', 'P', 'E']
  while (size >= 1000 && order + 1 < units.length) {
    order += 1
    size /= 1000
  }
  size = Math.round(size)
  let prefix = units[order]
  return `${size}${prefix}B`
}

@customElement('file-item')
export class FileItem extends LitElement {
  @property()
  stat: FileStat
  @property()
  selected: boolean

  protected createRenderRoot(): HTMLElement {
    return this
  }

  _onAnchorClick(event: Event) {
    console.log('click', event)
    event.preventDefault()
    window.history.pushState({}, '', event.target.href)
    window.dispatchEvent(new Event('routechange'))
  }

  override render() {
    let size = this.stat.type == 'file' ? formatFilesize(this.stat.size) : ''
    let href = this.stat.type == 'file' ? join('/dav', this.stat.filename) : `/frontend/${this.stat.filename}`
    return html`
      <td><input type="checkbox" .checked=${this.selected} @input=${e => this.selected = e.target.checked} /></td>
      <td>
        <a href=${href} @click=${this._onAnchorClick}>${this.stat.basename}</a>
      </td>
      <td>
      </td>
      <td>${size}</td>
      <td>${this.stat.lastmod}</td>
    `
  }
}

@customElement('file-browser')
export class FileBrowser extends LitElement {
  constructor() {
    super()
    this.fetchContents()
  }

  @property()
  path: string | null = null
  @property()
  pathSegments: string[] | null = null

  @property()
  entries: Array<FileStat> = []

  protected shouldUpdate(changedProperties: PropertyValues): boolean {
    if (changedProperties.has('path')) {
      this.fetchContents()
      this.pathSegments = this.path.split('/')
    }
    return true
  }

  async fetchContents() {
    if (this.path === null) return
    this.entries = await davClient.getDirectoryContents(this.path, { details: false }) as FileStat[]
    console.log(this.entries)
  }

  protected createRenderRoot(): HTMLElement {
    return this
  }

  _onAnchorClick(event: Event) {
    console.log('click', event)
    event.preventDefault()
    window.history.pushState({}, '', event.target.href)
    window.dispatchEvent(new Event('routechange'))
  }

  override render() {
    return html`
      <h1>File Browser</h1>
      <pre>${this.path}</pre>
      <table>
        <tr>
          <th></th>
          <th>Name</th>
          <th><!-- Actions --></th>
          <th>Size</th>
          <th>Modified</th>
        </tr>
        <tr>
          <td></td>
          <td>
            <a href="./" @click=${this._onAnchorClick} >..</a>
          </td>
          <td></td>
          <td></td>
          <td></td>
        </tr>
        ${this.entries.map(entry => html`<file-item .stat=${entry}/>`)}
      </table>
      `
  }
}

