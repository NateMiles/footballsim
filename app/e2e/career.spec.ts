import { expect, test, type Page } from '@playwright/test';

const shots = (name: string, project: string) => `e2e/screenshots/${project}-${name}.png`;

async function newCareer(page: Page) {
  await page.goto('./');
  await page.getByRole('button', { name: 'New game' }).click();
  await page.getByRole('spinbutton').fill('7');
  await page.getByRole('button', { name: 'Generate' }).click();
  await expect(page.getByRole('heading', { name: 'Choose your club' })).toBeVisible();
  await page.locator('.club').nth(3).click();
  await page.getByLabel('First name').fill('Robin');
  await page.getByLabel('Last name').fill('Hale');
  await page.getByRole('button', { name: /Take charge of/ }).click();
  await expect(page.getByText('Robin Hale')).toBeVisible();
}

test('a season can be started, managed and saved', async ({ page }, info) => {
  const project = info.project.name;
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));

  await newCareer(page);
  await page.screenshot({ path: shots('home', project) });

  await page.getByRole('link', { name: /Squad/ }).click();
  await expect(page.getByRole('heading', { name: 'Squad' })).toBeVisible();
  await expect(page.locator('tbody tr')).not.toHaveCount(0);
  await page.screenshot({ path: shots('squad', project) });

  await page.locator('tbody tr').first().click();
  await expect(page.getByRole('heading', { name: 'Technical' })).toBeVisible();
  await page.screenshot({ path: shots('player', project) });

  await page.getByRole('link', { name: /Tactics/ }).click();
  await expect(page.getByRole('heading', { name: 'Formation' })).toBeVisible();
  await page.getByRole('button', { name: '4-3-3' }).click();
  await page.getByRole('button', { name: 'Save tactic' }).click();
  await expect(page.getByText('Your tactic')).toBeVisible();
  await page.screenshot({ path: shots('tactics', project) });

  // Training: a schedule and one player's own training.
  await page.goto('./#/training');
  await expect(page.getByRole('heading', { name: 'Formations' })).toBeVisible();
  await page.getByRole('radio', { name: /Attacking/ }).click();
  await page.getByRole('radio', { name: 'Hard' }).click();
  await expect(page.getByTestId('effects')).toContainText('Growth +12%');
  await page.locator('tbody select').first().selectOption('p:WBL');
  await expect(page.locator('tbody tr').first()).toContainText('/20');
  await page.screenshot({ path: shots('training', project), fullPage: true });

  // The new club management screens.
  await page.goto('./#/finances');
  await expect(page.getByRole('heading', { name: 'Finances' })).toBeVisible();
  await expect(page.getByText('Transfer budget')).toBeVisible();
  await page.goto('./#/staff');
  await expect(page.getByRole('heading', { name: 'Your staff' })).toBeVisible();
  await page.screenshot({ path: shots('staff', project) });
  await page.goto('./#/transfers/search');
  await page.getByLabel('Position').selectOption('ST');
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await expect(page.locator('tbody tr').first()).toBeVisible();
  await page.screenshot({ path: shots('transfers', project) });
  await page.locator('tbody tr').first().click();
  await expect(page.getByRole('heading', { name: 'Market' })).toBeVisible();
  await page.getByRole('button', { name: 'Scout this player' }).click();
  await expect(page.getByText('Being scouted')).toBeVisible();
  await page.screenshot({ path: shots('scouted-player', project) });
  await page.getByRole('button', { name: 'Make offer' }).click();
  await expect(page.getByText(/Your offer|budget is|transfer budget/i).first()).toBeVisible();
  await page.goto('./#/transfers/offers');
  await expect(page.getByRole('heading', { name: 'Your offers' })).toBeVisible();

  await page.getByRole('link', { name: /Table/ }).click();
  await expect(page.locator('tbody tr')).toHaveCount(20);
  await page.getByRole('button', { name: 'Season odds' }).click();
  await expect(page.getByText(/simulated 2,000 times/)).toBeVisible();
  await page.screenshot({ path: shots('odds', project) });

  // Preparing for the first match: the opposition and the predictor.
  await page.goto('./#/prepare');
  await expect(page.getByRole('heading', { name: 'Likely eleven' })).toBeVisible();
  await page.getByRole('button', { name: 'Predict with your tactic' }).click();
  await expect(page.getByText(/Win · draw · lose/)).toBeVisible();
  await page.screenshot({ path: shots('prepare', project), fullPage: true });

  // Cups: the national cup's first round is drawn with the season.
  await page.goto('./#/cup');
  await expect(page.getByText(/Still to come/)).toBeVisible();
  await expect(page.locator('details.stage').first()).toBeVisible();
  await page.screenshot({ path: shots('cup', project) });
  await page.goto('./#/competitions');
  await expect(page.getByRole('heading', { name: 'Competitions' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Nations' })).toBeVisible();
  await page.screenshot({ path: shots('competitions', project) });
  await page.goto('./#/fixtures');
  await expect(page.getByText('All competitions')).toBeVisible();

  // Continue to the first match day.
  for (let i = 0; i < 20 && !page.url().includes('#/match'); i++) {
    await page.getByRole('button', { name: /Continue|Match day/ }).click();
    await page.waitForFunction(
      () => location.hash.includes('match') || document.querySelector('button.continue:not([disabled])') !== null,
    );
  }
  await expect(page.getByRole('heading', { name: 'Match day' })).toBeVisible();
  await page.getByRole('button', { name: /Kick off/ }).click();
  await page.getByLabel('Speed').selectOption('32');
  await expect(page.getByTestId('clock')).toHaveText(/^(1[0-9]|[2-9][0-9])'$/, { timeout: 60_000 });
  await page.getByRole('tab', { name: 'Team' }).click();
  await page.locator('.panel .p').nth(9).click();
  await page.locator('.panel .p').filter({ hasNotText: /^$/ }).last().click();
  await expect(page.getByRole('status').filter({ hasText: /replaces/ })).toBeVisible();
  await page.getByRole('button', { name: /Resume/ }).click();
  await page.getByRole('tab', { name: 'Commentary' }).click();
  await expect(page.getByTestId('commentary').locator('li')).not.toHaveCount(0);
  await page.screenshot({ path: shots('match', project) });

  await page.getByRole('button', { name: 'Skip to end' }).click();
  await expect(page.getByRole('heading', { name: /ratings/ }).first()).toBeVisible();
  await page.screenshot({ path: shots('report', project) });
  await page.getByRole('button', { name: 'Continue' }).click();
  await expect(page.getByRole('heading', { name: 'Last match' })).toBeVisible();

  // Save, reload, and load it back.
  await page.getByRole('link', { name: /Saves/ }).click();
  await page.getByLabel('Save name').fill('e2e');
  await page.getByRole('button', { name: 'Save game' }).click();
  await expect(page.getByText('Saved as “e2e”.')).toBeVisible();
  await page.goto('./');
  await page.getByRole('button', { name: /· e2e$/ }).click();
  await expect(page.getByRole('heading', { name: 'Last match' })).toBeVisible();
  await expect(page.getByText(/Robin Hale/)).toBeVisible();

  expect(errors).toEqual([]);
});
