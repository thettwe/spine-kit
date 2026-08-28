# INT-042: Invoice totals include tax
Owner: @alice · Template: intent@2 · Ticket: https://tracker.example.com/T-1187 · Constitution: v3

## Goal (2–3 sentences)
Invoices show a tax-inclusive total, so finance stops reconciling two numbers by
hand. The total is computed from the line items the invoice already lists, and no
invoice that has already been issued changes retroactively.

## Non-goals (mandatory, minimum 2)
- Multi-jurisdiction tax rules. One rate, from the customer's billing country.
- Recalculating invoices that were already issued.
- A tax report or an export of one. Reporting is its own intent.

## Acceptance criteria (maximum 6 — more means split the task)
AC-1: Given an invoice with taxable lines, when it is rendered, then the total
  includes tax at the customer's rate.
AC-2: Given an invoice whose lines are all zero-rated, when it is rendered, then
  the tax line reads 0.00 and the total equals the subtotal.
AC-3: Given an invoice issued before this ships, when it is re-rendered, then its
  stored total is unchanged.

## Touchpoints (expected blast radius)
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/

## Open questions (optional — must be empty before implementation)
