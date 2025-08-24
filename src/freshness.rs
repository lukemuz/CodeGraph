use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};
use std::fs;
use anyhow::Result;
use tracing::{info, debug};
use tokio::time::interval;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct FreshnessManager {
    index_path: PathBuf,
    project_path: PathBuf,
    last_check: SystemTime,
    check_interval: Duration,
    // Sample size for staleness check (check N random files)
    sample_size: usize,
}

impl FreshnessManager {
    pub fn new(index_path: PathBuf, project_path: PathBuf) -> Self {
        Self {
            index_path,
            project_path,
            last_check: SystemTime::now(),
            check_interval: Duration::from_secs(300), // 5 minutes default
            sample_size: 10,
        }
    }
    
    pub fn with_interval(mut self, seconds: u64) -> Self {
        self.check_interval = Duration::from_secs(seconds);
        self
    }
    
    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.sample_size = size;
        self
    }
    
    /// Check if index is stale by sampling files
    pub fn is_stale(&self) -> Result<bool> {
        // Get index modification time
        let index_meta = fs::metadata(&self.index_path)?;
        let index_time = index_meta.modified()?;
        
        debug!("Checking index staleness, last modified: {:?}", index_time);
        
        // Collect a sample of Python files to check
        let mut python_files = Vec::new();
        for entry in walkdir::WalkDir::new(&self.project_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "py"))
        {
            python_files.push(entry.path().to_path_buf());
            if python_files.len() >= self.sample_size * 3 {
                break; // Collect more than we need for randomization
            }
        }
        
        // Check a random sample
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        python_files.shuffle(&mut rng);
        
        for file in python_files.iter().take(self.sample_size) {
            if let Ok(file_meta) = fs::metadata(file) {
                if let Ok(file_time) = file_meta.modified() {
                    if file_time > index_time {
                        info!("Found stale file: {} (modified after index)", file.display());
                        return Ok(true);
                    }
                }
            }
        }
        
        debug!("Index appears fresh after checking {} files", self.sample_size);
        Ok(false)
    }
    
    /// Check if enough time has passed since last check
    pub fn should_check(&self) -> bool {
        match SystemTime::now().duration_since(self.last_check) {
            Ok(elapsed) => elapsed >= self.check_interval,
            Err(_) => true, // If time went backwards, check anyway
        }
    }
    
    /// Update the last check time
    pub fn mark_checked(&mut self) {
        self.last_check = SystemTime::now();
    }
    
    /// Start a background task that periodically checks freshness
    pub fn start_background_refresh(
        manager: Arc<Mutex<Self>>,
        rebuild_callback: Arc<dyn Fn() + Send + Sync>,
    ) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Check every minute
            
            loop {
                interval.tick().await;
                
                let should_check = {
                    let mgr = manager.lock().await;
                    mgr.should_check()
                };
                
                if should_check {
                    let is_stale = {
                        let mut mgr = manager.lock().await;
                        mgr.mark_checked();
                        mgr.is_stale().unwrap_or(false)
                    };
                    
                    if is_stale {
                        info!("Periodic check found stale index, triggering rebuild");
                        rebuild_callback();
                    }
                }
            }
        });
    }
}

/// Quick staleness check for on-demand validation
pub async fn quick_staleness_check(index_path: &Path, project_path: &Path) -> Result<bool> {
    // Just check if any .py file is newer than the index
    let index_time = fs::metadata(index_path)?.modified()?;
    
    // Quick check: look at just the top-level Python files
    for entry in fs::read_dir(project_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().map_or(false, |ext| ext == "py") {
            if let Ok(meta) = entry.metadata() {
                if meta.modified()? > index_time {
                    return Ok(true);
                }
            }
        }
    }
    
    Ok(false)
}