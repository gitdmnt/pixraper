<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Temporal } from "@js-temporal/polyfill";
  let detailedMode = false;
  let scrapingOption = {
    tags: [] as string[],
    searchMode: "s_tag",
    scd: Temporal.Now.plainDateISO().toString(),
    ecd: Temporal.Now.plainDateISO().toString(),
  };

  const addTag = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const input = e.target as HTMLInputElement;
      const newTag = input.value.trim();
      if (newTag && !scrapingOption.tags.includes(newTag)) {
        scrapingOption.tags = [...scrapingOption.tags, newTag];
        input.value = "";
      }
      console.log("Tags:", scrapingOption.tags);
    }
  };

  const removeTag = (tag: string) => {
    scrapingOption.tags = scrapingOption.tags.filter((t) => t !== tag);
    console.log("Tags:", scrapingOption.tags);
  };

  const startScraping = () => {
    if (detailedMode) {
      invoke("start_detailed_scraping", { scrapingOption })
        .then(() => {
          console.log("Detailed scraping started");
        })
        .catch((error) => {
          console.error("Error starting detailed scraping:", error);
        });
    } else {
      invoke("start_rough_scraping", { scrapingOption })
        .then(() => {
          console.log("Scraping started");
        })
        .catch((error) => {
          console.error("Error starting scraping:", error);
        });
    }
    console.log("Starting scraping with options:", scrapingOption);
  };

  const stopScraping = () => {
    invoke("stop_scraping").then((message) => {
      console.log(message);
    });
  };
</script>

<div class="container">
  <div class="scraping-option-container">
    <div class="detailed-mode">
      <label for="detailedMode">Detailed Mode</label>
      <input type="checkbox" id="detailedMode" bind:checked={detailedMode} />
    </div>
    <div class="search-mode">
      <span>Search Mode</span>
      <select bind:value={scrapingOption.searchMode}>
        <option value="s_tag">Partially Matching Tags</option>
        <option value="s_tag_full">Full Tag Search</option>
        <option value="s_tc">Title and Caption Search</option>
      </select>
    </div>
    <div class="tags-input">
      <label for="tags">Tags to Scrape</label>
      <div class="tags">
        {#each scrapingOption.tags as tag}
          <span class="tag">
            <span>{tag}</span>
            <button onclick={() => removeTag(tag)}>x</button>
          </span>
        {/each}
        <div class="add-tag">
          <input
            type="text"
            placeholder="Enter tag and press Enter"
            onkeydown={addTag}
          />
        </div>
      </div>
    </div>
    <div class="scd-ecd">
      <label for="scd">Start Date:</label>
      <input type="date" id="scd" bind:value={scrapingOption.scd} />
      <label for="ecd">End Date:</label>
      <input type="date" id="ecd" bind:value={scrapingOption.ecd} />
    </div>
  </div>

  <div class="scraping-progress">
    <h3>Scraping Progress</h3>
    <div class="progress-bar">
      <div class="progress-fill" style="width: 0%"></div>
    </div>
    <p class="progress-text">0 / 0 items scraped</p>
  </div>

  <button class="scrape-button" onclick={startScraping}>Start Scraping</button>
  <button class="stop-button" onclick={stopScraping}>Stop Scraping</button>
</div>
