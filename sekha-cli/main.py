import click
from rich.console import Console
from rich.table import Table
from sekha import SekhaClient, ClientConfig

console = Console()

@click.group()
@click.option('--api-url', default='http://localhost:8080', envvar='SEKHA_API_URL')
@click.option('--api-key', envvar='SEKHA_API_KEY')
@click.pass_context
def cli(ctx, api_url, api_key):
    """Sekha AI Memory Controller CLI"""
    ctx.ensure_object(dict)
    ctx.obj['client'] = SekhaClient(
        ClientConfig(base_url=api_url, api_key=api_key)
    )

@cli.command()
@click.argument('query')
@click.option('--label', help='Filter by label')
@click.option('--limit', default=10, type=int)
@click.pass_context
def query(ctx, query, label, limit):
    """Search conversations"""
    client = ctx.obj['client']
    results = client.smart_query(query=query, labels=[label] if label else None)
    
    table = Table(title=f"Search Results: {query}")
    table.add_column("ID", style="cyan")
    table.add_column("Label", style="magenta")
    table.add_column("Score", style="green")
    
    for result in results[:limit]:
        table.add_row(
            str(result.id)[:8],
            result.label,
            f"{result.score:.2f}"
        )
    
    console.print(table)

@cli.command()
@click.argument('id')
@click.option('--format', default='markdown', type=click.Choice(['markdown', 'json']))
@click.pass_context
def show(ctx, id, format):
    """Show conversation details"""
    client = ctx.obj['client']
    conv = client.get_conversation(id)
    
    if format == 'markdown':
        console.print(f"# {conv.label}\n")
        for msg in conv.messages:
            console.print(f"**{msg.role}**: {msg.content}\n")
    else:
        console.print_json(data=conv.dict())

@cli.command()
@click.option('--label', help='Export only this label')
@click.option('--output', '-o', type=click.File('w'), default='-')
@click.option('--format', default='markdown', type=click.Choice(['markdown', 'json']))
@click.pass_context
def export(ctx, label, output, format):
    """Export conversations"""
    client = ctx.obj['client']
    content = client.export(label=label, format=format)
    output.write(content)

@cli.command()
@click.option('--dry-run', is_flag=True, help='Show what would be pruned')
@click.pass_context
def prune(ctx, dry_run):
    """Get pruning suggestions"""
    client = ctx.obj['client']
    suggestions = client.get_pruning_suggestions()
    
    if dry_run:
        console.print("[yellow]Would prune:[/yellow]")
        for s in suggestions:
            console.print(f"  - {s.conversation_id}: {s.reason}")
    else:
        console.print("[red]Use --dry-run to see suggestions[/red]")

@cli.command()
@click.argument('file', type=click.File('r'))
@click.option('--label', default='Imported', help='Label for imported conversation')
@click.pass_context
def store(ctx, file, label):
    """Import conversation from JSON file"""
    import json
    client = ctx.obj['client']
    
    data = json.load(file)
    conv = client.create_conversation(
        messages=data['messages'],
        label=label,
        folder=data.get('folder', '/imported')
    )
    
    console.print(f"[green]Imported conversation: {conv.id}[/green]")

@cli.group()
def labels():
    """Manage conversation labels"""

@labels.command('list')
@click.pass_context
def list_labels(ctx):
    """List all labels"""
    client = ctx.obj['client']
    conversations = client.list_conversations()
    
    labels = sorted(set(c.label for c in conversations))
    for label in labels:
        console.print(f"  - {label}")

if __name__ == '__main__':
    cli()