import { html, LitElement, PropertyValues } from "lit";
import { customElement, property } from "lit/decorators.js";
import { davClient } from "./dav-client.ts";
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
    navigation!.addEventListener('navigate', this._onNavigate)
  }

  disconnectedCallback(): void {
    super.disconnectedCallback()
    navigation!.removeEventListener('navigate', this._onNavigate)
  }

  updateRoute = () => {
    this.path = globalThis.location.pathname.replace(/^\/frontend/, '')
  }

  _onNavigate = (event: NavigateEvent) => {
    if (!event.canIntercept || event.hashChange || event.downloadRequest !== null) {
      return;
    }

    event.intercept({
      handler: async () => {
        this.updateRoute()
        globalThis.dispatchEvent(new Event('routechange'))
      }
    })
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
  const units = ['', 'K', 'M', 'G', 'T', 'P', 'E']
  while (size >= 1000 && order + 1 < units.length) {
    order += 1
    size /= 1000
  }
  size = Math.round(size)
  const prefix = units[order]
  return `${size}${prefix}B`
}

@customElement('file-item')
export class FileItem extends LitElement {
  @property()
  stat!: FileStat
  @property()
  selected: boolean = false

  protected createRenderRoot(): HTMLElement {
    return this
  }

  override render() {
    const size = this.stat.type == 'file' ? formatFilesize(this.stat.size) : ''
    const href = this.stat.type == 'file' ? join('/dav', this.stat.filename) : `/frontend${this.stat.filename}`

    return html`
      <td><input type="checkbox" .checked=${this.selected} @input=${(e: InputEvent) => this.selected = (e.target! as HTMLInputElement).checked} /></td>
      <td>
        <a href=${href}>${this.stat.basename}</a>
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
  path!: string

  @property()
  entries: Array<FileStat> = []

  protected shouldUpdate(changedProperties: PropertyValues): boolean {
    if (changedProperties.has('path')) {
      this.fetchContents()
    }
    return true
  }

  async fetchContents() {
    if (this.path === null) return
    const entries = await davClient.getDirectoryContents(this.path, { details: false }) as FileStat[]
    this.entries = entries.filter(item => item.filename !== this.path)
  }

  protected createRenderRoot(): HTMLElement {
    return this
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
            <a href="./">..</a>
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

